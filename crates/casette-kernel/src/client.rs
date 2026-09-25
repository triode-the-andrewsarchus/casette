//! Asynchronous process manager and JSONL protocol client for SageMath.
//!
//! Provides [`SageKernel`], managing the background `worker.py` process lifecycle,
//! serial request scheduling, process-group isolation, interrupts via `SIGINT`,
//! and clean session restarts.

use crate::discovery::SageDiscovery;
use crate::error::KernelError;
use crate::protocol::{SymbolPayload, WorkerRequest, WorkerResponse};
use nix::sys::signal::{self, SigHandler, SigSet, SigmaskHow, Signal};
use nix::unistd::Pid;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, error, info, warn};

/// Standard calculator variables pre-declared in the SageMath session at boot.
pub const BOOT_VARIABLE_NAMES: &[&str] = &[
    "u", "v", "w", "x", "y", "z", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "y1", "y2",
    "y3", "y4", "y5", "y6", "y7", "y8", "y9", "t",
];

/// Generates the SageMath `var(...)` declaration statement for the boot variables.
#[must_use]
pub fn boot_prelude() -> String {
    format!("var('{}')", BOOT_VARIABLE_NAMES.join(", "))
}

/// Configuration options for launching a [`SageKernel`].
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Path to the Python executable capable of importing SageMath.
    pub python_path: PathBuf,

    /// Filesystem path to `worker.py`.
    pub worker_script: PathBuf,

    /// Maximum time allowed for the worker to start up and emit its ready envelope.
    pub startup_timeout: Duration,

    /// Default timeout for individual code evaluation requests.
    pub default_eval_timeout: Duration,

    /// Optional initial session precision digits (default: 10).
    pub initial_precision: Option<u32>,

    /// Whether to evaluate the standard boot prelude (`var('u, v, w, x, ...')`) on launch.
    pub run_boot_prelude: bool,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            python_path: PathBuf::from("python3"),
            worker_script: PathBuf::from("v0/01-worker-protocol/worker.py"),
            startup_timeout: Duration::from_secs(30),
            default_eval_timeout: Duration::from_secs(60),
            initial_precision: Some(10),
            run_boot_prelude: true,
        }
    }
}

enum KernelCommand {
    Eval {
        request: WorkerRequest,
        timeout: Duration,
        reply: oneshot::Sender<Result<WorkerResponse, KernelError>>,
    },
    Ping {
        request: WorkerRequest,
        timeout: Duration,
        start_time: Instant,
        reply: oneshot::Sender<Result<Duration, KernelError>>,
    },
    Symbols {
        request: WorkerRequest,
        timeout: Duration,
        reply: oneshot::Sender<Result<Vec<SymbolPayload>, KernelError>>,
    },
    ConfigPrecision {
        request: WorkerRequest,
        timeout: Duration,
        reply: oneshot::Sender<Result<u32, KernelError>>,
    },
    Restart {
        reply: oneshot::Sender<Result<(), KernelError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), KernelError>>,
    },
}

/// Active connection handle to a background SageMath worker process.
#[derive(Clone)]
pub struct SageKernel {
    command_tx: mpsc::Sender<KernelCommand>,
    worker_pid: Arc<AtomicU32>,
    is_running: Arc<AtomicBool>,
    next_req_id: Arc<AtomicU64>,
    default_timeout: Duration,
}

impl SageKernel {
    /// Launches a new SageMath kernel using automatically discovered system paths.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if Sage discovery fails or the worker process fails to boot.
    pub async fn launch_discovered() -> Result<Self, KernelError> {
        let env = SageDiscovery::discover().await?;
        Self::launch(KernelConfig {
            python_path: env.python_path,
            worker_script: env.worker_script,
            ..Default::default()
        })
        .await
    }

    /// Launches a new SageMath kernel with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::SpawnFailed`] if the process cannot be started or times out
    /// waiting for the ready handshake.
    pub async fn launch(config: KernelConfig) -> Result<Self, KernelError> {
        let (command_tx, command_rx) = mpsc::channel(32);
        let worker_pid = Arc::new(AtomicU32::new(0));
        let is_running = Arc::new(AtomicBool::new(false));
        let next_req_id = Arc::new(AtomicU64::new(1));

        let mut actor = WorkerActor::new(
            config.clone(),
            command_rx,
            Arc::clone(&worker_pid),
            Arc::clone(&is_running),
        );

        // Perform initial boot synchronously before returning kernel handle
        actor.boot().await?;

        // Spawn background management loop
        tokio::spawn(async move {
            actor.run().await;
        });

        let kernel = Self {
            command_tx,
            worker_pid,
            is_running,
            next_req_id,
            default_timeout: config.default_eval_timeout,
        };

        // If an initial precision was specified, configure it now
        if let Some(digits) = config.initial_precision {
            let _ = kernel.config_precision(digits).await;
        }

        Ok(kernel)
    }

    /// Evaluates a SageMath code snippet and awaits its structured result envelope.
    ///
    /// # Arguments
    ///
    /// * `code` - Sage source code to evaluate.
    /// * `numeric` - If true, formats result with numerical approximation as primary.
    /// * `precision_digits` - Optional precision override for this evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if the worker crashes, the request times out, or the
    /// kernel is shut down. Note that normal mathematical errors (such as `ZeroDivisionError`)
    /// are returned inside [`WorkerResponse`] with `ok = false`.
    pub async fn eval(
        &self,
        code: impl Into<String>,
        numeric: bool,
        precision_digits: Option<u32>,
    ) -> Result<WorkerResponse, KernelError> {
        let id = format!("req-{}", self.next_req_id.fetch_add(1, Ordering::Relaxed));
        let request = WorkerRequest::eval(id, code, numeric, precision_digits);
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(KernelCommand::Eval {
                request,
                timeout: self.default_timeout,
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Sends a ping probe to measure worker responsiveness and roundtrip latency.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if the worker is unreachable or unresponsive.
    pub async fn ping(&self) -> Result<Duration, KernelError> {
        let id = format!("p-{}", self.next_req_id.fetch_add(1, Ordering::Relaxed));
        let request = WorkerRequest::ping(id);
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(KernelCommand::Ping {
                request,
                timeout: Duration::from_secs(10),
                start_time: Instant::now(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Introspects the current SageMath session for user-defined symbols and functions.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if the symbols query fails.
    pub async fn symbols(&self) -> Result<Vec<SymbolPayload>, KernelError> {
        let id = format!("sym-{}", self.next_req_id.fetch_add(1, Ordering::Relaxed));
        let request = WorkerRequest::symbols(id);
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(KernelCommand::Symbols {
                request,
                timeout: Duration::from_secs(15),
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Sets the session default precision digits for numeric approximations.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if the configuration request fails.
    pub async fn config_precision(&self, digits: u32) -> Result<u32, KernelError> {
        let id = format!("cfg-{}", self.next_req_id.fetch_add(1, Ordering::Relaxed));
        let request = WorkerRequest::config_precision(id, digits);
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(KernelCommand::ConfigPrecision {
                request,
                timeout: Duration::from_secs(10),
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Interrupts the currently executing evaluation by sending `SIGINT` directly
    /// to the worker process PID.
    ///
    /// This immediately triggers a `KeyboardInterrupt` inside Python/cysignals,
    /// aborting long-running calculations without terminating the worker process.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::SignalFailed`] if the signal could not be delivered.
    pub fn interrupt(&self) -> Result<(), KernelError> {
        let pid = self.worker_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Ok(());
        }

        debug!("sending SIGINT to worker pid {}", pid);
        signal::kill(Pid::from_raw(pid as i32), Signal::SIGINT).map_err(|err| {
            KernelError::SignalFailed {
                pid,
                reason: err.to_string(),
            }
        })
    }

    /// Restarts the SageMath worker process, clearing the active session namespace.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if restarting fails.
    pub async fn restart(&self) -> Result<(), KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(KernelCommand::Restart { reply: reply_tx })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Shuts down the background worker process cleanly.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if shutdown communication fails.
    pub async fn shutdown(&self) -> Result<(), KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(KernelCommand::Shutdown { reply: reply_tx })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Returns `true` if the kernel is currently connected to an active worker process.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Returns the active operating system PID of the worker process, if running.
    #[must_use]
    pub fn worker_pid(&self) -> Option<u32> {
        let pid = self.worker_pid.load(Ordering::Relaxed);
        if pid > 0 {
            Some(pid)
        } else {
            None
        }
    }
}

struct WorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout_lines: Lines<BufReader<ChildStdout>>,
    stderr_buffer: Arc<Mutex<VecDeque<String>>>,
    reported_pid: u32,
}

impl WorkerProcess {
    async fn spawn(config: &KernelConfig) -> Result<Self, KernelError> {
        debug!(
            "spawning Sage worker: {} {}",
            config.python_path.display(),
            config.worker_script.display()
        );

        let mut cmd = Command::new(&config.python_path);
        cmd.arg(&config.worker_script)
            .process_group(0) // Separate process group (setsid)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Problem 010: Reset signal dispositions and unmask signals so child worker
        // does not inherit parent's SIG_IGN or blocked mask, allowing Python/cysignals
        // to install SIGINT handlers reliably.
        unsafe {
            cmd.pre_exec(|| {
                let empty_set = SigSet::empty();
                let _ = signal::sigprocmask(SigmaskHow::SIG_SETMASK, Some(&empty_set), None);
                let _ = signal::signal(Signal::SIGINT, SigHandler::SigDfl);
                Ok(())
            });
        }

        let mut child = cmd.spawn().map_err(|e| {
            KernelError::SpawnFailed(format!(
                "failed to launch '{}': {}",
                config.python_path.display(),
                e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| KernelError::SpawnFailed("failed to capture child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| KernelError::SpawnFailed("failed to capture child stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| KernelError::SpawnFailed("failed to capture child stderr".into()))?;

        let stdout_lines = BufReader::new(stdout).lines();
        let stderr_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(64)));

        // Spawn background stderr drain task
        let stderr_buf_clone = Arc::clone(&stderr_buffer);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut buf = stderr_buf_clone.lock().await;
                if buf.len() >= 64 {
                    buf.pop_front();
                }
                buf.push_back(line);
            }
        });

        let mut worker = Self {
            child,
            stdin,
            stdout_lines,
            stderr_buffer,
            reported_pid: 0,
        };

        // Await ready banner envelope
        let ready_resp = worker
            .read_response(config.startup_timeout)
            .await
            .map_err(|e| {
                KernelError::SpawnFailed(format!(
                    "timed out or failed waiting for ready banner: {}",
                    e
                ))
            })?;

        if !ready_resp.is_ready() {
            return Err(KernelError::SpawnFailed(format!(
                "expected ready banner envelope, received: {:?}",
                ready_resp
            )));
        }

        let pid = ready_resp
            .pid
            .or_else(|| worker.child.id())
            .ok_or_else(|| KernelError::SpawnFailed("worker did not report a valid PID".into()))?;

        worker.reported_pid = pid;
        info!("Sage worker ready: reported PID={}", pid);

        Ok(worker)
    }

    async fn send_request(&mut self, request: &WorkerRequest) -> Result<(), KernelError> {
        let mut line = serde_json::to_string(request)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_response(&mut self, timeout: Duration) -> Result<WorkerResponse, KernelError> {
        let line_result = tokio::time::timeout(timeout, self.stdout_lines.next_line()).await;

        match line_result {
            Ok(Ok(Some(line))) => {
                let resp: WorkerResponse = serde_json::from_str(&line)?;
                Ok(resp)
            }
            Ok(Ok(None)) => {
                let exit_status = self.child.try_wait().ok().flatten();
                let stderr_logs = self.recent_stderr().await;
                Err(KernelError::WorkerExited {
                    code: exit_status.and_then(|s| s.code()),
                    stderr: stderr_logs,
                })
            }
            Ok(Err(io_err)) => Err(KernelError::Io(io_err)),
            Err(_) => Err(KernelError::RequestTimeout {
                id: "unknown".into(),
                duration: timeout,
            }),
        }
    }

    async fn recent_stderr(&self) -> String {
        let buf = self.stderr_buffer.lock().await;
        buf.iter().cloned().collect::<Vec<_>>().join("\n")
    }

    async fn kill_and_reap(&mut self) {
        if let Some(pid) = self.child.id() {
            // Problem 024: Kill the entire process group (-pid) because `cmd.process_group(0)`
            // established a new process group with this child as group leader. If python was
            // launched via a wrapper script (e.g. `sage -python`), this ensures both the
            // wrapper script and its child Python worker are cleanly reaped without leaking orphans.
            let _ = signal::kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);
        }
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

struct WorkerActor {
    config: KernelConfig,
    command_rx: mpsc::Receiver<KernelCommand>,
    worker: Option<WorkerProcess>,
    worker_pid: Arc<AtomicU32>,
    is_running: Arc<AtomicBool>,
}

impl WorkerActor {
    fn new(
        config: KernelConfig,
        command_rx: mpsc::Receiver<KernelCommand>,
        worker_pid: Arc<AtomicU32>,
        is_running: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            command_rx,
            worker: None,
            worker_pid,
            is_running,
        }
    }

    async fn boot(&mut self) -> Result<(), KernelError> {
        let mut worker = WorkerProcess::spawn(&self.config).await?;
        self.worker_pid
            .store(worker.reported_pid, Ordering::Relaxed);
        self.is_running.store(true, Ordering::Relaxed);

        if self.config.run_boot_prelude {
            debug!("evaluating boot prelude: {}", boot_prelude());
            let prelude_req = WorkerRequest::eval("boot-prelude", boot_prelude(), false, None);
            if let Err(e) = worker.send_request(&prelude_req).await {
                warn!("failed to send boot prelude: {}", e);
            } else {
                match worker.read_response(self.config.startup_timeout).await {
                    Ok(resp) => {
                        if !resp.ok {
                            warn!("boot prelude returned non-ok: {:?}", resp.error);
                        }
                    }
                    Err(e) => {
                        warn!("failed to read boot prelude response: {}", e);
                    }
                }
            }
        }

        self.worker = Some(worker);
        Ok(())
    }

    async fn restart(&mut self) -> Result<(), KernelError> {
        info!("restarting Sage worker process");
        if let Some(mut w) = self.worker.take() {
            let req = WorkerRequest::shutdown("shutdown-restart");
            let _ = w.send_request(&req).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            w.kill_and_reap().await;
        }
        self.worker_pid.store(0, Ordering::Relaxed);
        self.is_running.store(false, Ordering::Relaxed);
        self.boot().await
    }

    async fn shutdown(&mut self) {
        info!("shutting down Sage worker process");
        if let Some(mut w) = self.worker.take() {
            let req = WorkerRequest::shutdown("shutdown");
            let _ = w.send_request(&req).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            w.kill_and_reap().await;
        }
        self.worker_pid.store(0, Ordering::Relaxed);
        self.is_running.store(false, Ordering::Relaxed);
    }

    async fn run(&mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                KernelCommand::Eval {
                    request,
                    timeout,
                    reply,
                } => {
                    let req_id = request.id.clone().unwrap_or_default();
                    let res = self.execute_request(&request, timeout, &req_id).await;
                    let _ = reply.send(res);
                }
                KernelCommand::Ping {
                    request,
                    timeout,
                    start_time,
                    reply,
                } => {
                    let req_id = request.id.clone().unwrap_or_default();
                    let res = self.execute_request(&request, timeout, &req_id).await;
                    let ping_res = res.map(|_| start_time.elapsed());
                    let _ = reply.send(ping_res);
                }
                KernelCommand::Symbols {
                    request,
                    timeout,
                    reply,
                } => {
                    let req_id = request.id.clone().unwrap_or_default();
                    let res = self.execute_request(&request, timeout, &req_id).await;
                    let sym_res = res.map(|resp| resp.symbols.unwrap_or_default());
                    let _ = reply.send(sym_res);
                }
                KernelCommand::ConfigPrecision {
                    request,
                    timeout,
                    reply,
                } => {
                    let req_id = request.id.clone().unwrap_or_default();
                    let res = self.execute_request(&request, timeout, &req_id).await;
                    let prec_res = res.and_then(|resp| {
                        resp.precision_digits.ok_or_else(|| {
                            KernelError::ProtocolViolation(
                                "config response missing precision_digits".into(),
                            )
                        })
                    });
                    let _ = reply.send(prec_res);
                }
                KernelCommand::Restart { reply } => {
                    let res = self.restart().await;
                    let _ = reply.send(res);
                }
                KernelCommand::Shutdown { reply } => {
                    self.shutdown().await;
                    let _ = reply.send(Ok(()));
                    break;
                }
            }
        }
        self.shutdown().await;
    }

    async fn execute_request(
        &mut self,
        request: &WorkerRequest,
        timeout: Duration,
        req_id: &str,
    ) -> Result<WorkerResponse, KernelError> {
        let worker = match self.worker.as_mut() {
            Some(w) => w,
            None => {
                // If not running, attempt to boot
                self.boot().await?;
                self.worker.as_mut().unwrap()
            }
        };

        if let Err(e) = worker.send_request(request).await {
            error!("failed to send request to worker: {}", e);
            self.is_running.store(false, Ordering::Relaxed);
            return Err(e);
        }

        match worker.read_response(timeout).await {
            Ok(resp) => Ok(resp),
            Err(KernelError::RequestTimeout { duration, .. }) => {
                warn!("request '{}' timed out, sending SIGINT", req_id);
                // Try interrupting the worker
                let pid = worker.reported_pid;
                let _ = signal::kill(Pid::from_raw(pid as i32), Signal::SIGINT);

                // Give it a brief window to emit interrupted envelope
                match worker.read_response(Duration::from_secs(3)).await {
                    Ok(resp) => Ok(resp),
                    Err(_) => {
                        error!("worker did not respond to SIGINT; restarting worker");
                        let _ = self.restart().await;
                        Err(KernelError::RequestTimeout {
                            id: req_id.to_string(),
                            duration,
                        })
                    }
                }
            }
            Err(KernelError::WorkerExited { code, stderr }) => {
                error!("worker process died: code={:?}, stderr={}", code, stderr);
                self.worker = None;
                self.worker_pid.store(0, Ordering::Relaxed);
                self.is_running.store(false, Ordering::Relaxed);
                Err(KernelError::WorkerExited { code, stderr })
            }
            Err(e) => Err(e),
        }
    }
}
