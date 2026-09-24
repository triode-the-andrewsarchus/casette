//! SageMath environment and worker script discovery for Linux systems.
//!
//! Provides automatic detection of local SageMath installations, validating
//! that the discovered Python interpreter can successfully import `sage.all`
//! and query the Sage version.

use crate::error::KernelError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Discovered SageMath runtime environment details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SageEnvironment {
    /// Path to the Python executable capable of executing Sage scripts.
    pub python_path: PathBuf,

    /// Discovered SageMath version string (e.g. `"10.9"` or `"SageMath version 10.9"`).
    pub sage_version: String,

    /// Absolute filesystem path to `worker.py`.
    pub worker_script: PathBuf,
}

/// Strategy for discovering and validating a local SageMath environment.
pub struct SageDiscovery;

impl SageDiscovery {
    /// Discovers the local SageMath runtime environment and `worker.py` location.
    ///
    /// Checks environment variables (`CASETTE_SAGE_PYTHON`, `CASETTE_SAGE_PATH`),
    /// followed by system Python candidates and Sage executables in PATH and standard
    /// system locations.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::DiscoveryFailed`] if no working Sage environment can be found
    /// or if the worker script cannot be located.
    pub async fn discover() -> Result<SageEnvironment, KernelError> {
        Self::discover_with_options(None, None).await
    }

    /// Discovers SageMath runtime with optional explicit path overrides.
    ///
    /// # Arguments
    ///
    /// * `custom_python` - Optional explicit path to a Python or Sage binary.
    /// * `custom_worker` - Optional explicit path to `worker.py`.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::DiscoveryFailed`] if the specified or discovered binary
    /// cannot run Sage or if the worker script does not exist.
    pub async fn discover_with_options(
        custom_python: Option<&Path>,
        custom_worker: Option<&Path>,
    ) -> Result<SageEnvironment, KernelError> {
        let worker_script = Self::locate_worker_script(custom_worker)?;

        // 1. Try explicit override if provided
        if let Some(path) = custom_python {
            if let Some(ver) = Self::probe_python_candidate(path).await {
                return Ok(SageEnvironment {
                    python_path: path.to_path_buf(),
                    sage_version: ver,
                    worker_script,
                });
            }
            return Err(KernelError::DiscoveryFailed(format!(
                "specified python binary at '{}' failed to import sage.all",
                path.display()
            )));
        }

        // 2. Check environment variable overrides
        if let Ok(env_val) = std::env::var("CASETTE_SAGE_PYTHON") {
            let p = PathBuf::from(env_val);
            if let Some(ver) = Self::probe_python_candidate(&p).await {
                return Ok(SageEnvironment {
                    python_path: p,
                    sage_version: ver,
                    worker_script,
                });
            }
        }

        if let Ok(env_val) = std::env::var("CASETTE_SAGE_PATH") {
            let p = PathBuf::from(env_val);
            if let Some(ver) = Self::probe_python_candidate(&p).await {
                return Ok(SageEnvironment {
                    python_path: p,
                    sage_version: ver,
                    worker_script,
                });
            }
        }

        // 3. Search standard Linux candidates
        let candidates = [
            PathBuf::from("python3"),
            PathBuf::from("/usr/bin/python3"),
            PathBuf::from("/usr/local/bin/python3"),
            PathBuf::from("/opt/sagemath/sage"),
            PathBuf::from("/usr/bin/sage"),
        ];

        for candidate in &candidates {
            if let Some(ver) = Self::probe_python_candidate(candidate).await {
                return Ok(SageEnvironment {
                    python_path: candidate.clone(),
                    sage_version: ver,
                    worker_script,
                });
            }
        }

        Err(KernelError::DiscoveryFailed(
            "could not find a working SageMath Python interpreter. Ensure 'sage' or a Python environment with 'sage.all' is installed.".into()
        ))
    }

    /// Probes a candidate Python or Sage binary to verify it can import `sage.all`.
    async fn probe_python_candidate(candidate: &Path) -> Option<String> {
        // Fast probe: run python3 -c "import sage.all, sage.version; print(sage.version.version)"
        let child = Command::new(candidate)
            .args([
                "-c",
                "import sage.all, sage.version; print(sage.version.version)",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let output = match child {
            Ok(c) => {
                match tokio::time::timeout(std::time::Duration::from_secs(10), c.wait_with_output())
                    .await
                {
                    Ok(Ok(out)) => out,
                    _ => return None,
                }
            }
            Err(_) => return None,
        };

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }

        None
    }

    /// Locates the `worker.py` script on disk.
    ///
    /// Searches explicit overrides, environment variables, relative paths from the current
    /// working directory, and standard install paths.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::DiscoveryFailed`] if `worker.py` cannot be found.
    pub fn locate_worker_script(custom_path: Option<&Path>) -> Result<PathBuf, KernelError> {
        if let Some(path) = custom_path {
            if path.exists() {
                return Ok(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
            }
            return Err(KernelError::DiscoveryFailed(format!(
                "explicit worker script path does not exist: '{}'",
                path.display()
            )));
        }

        if let Ok(env_val) = std::env::var("CASETTE_WORKER_SCRIPT") {
            let p = PathBuf::from(env_val);
            if p.exists() {
                return Ok(p.canonicalize().unwrap_or(p));
            }
        }

        let candidates = [
            PathBuf::from("v0/01-worker-protocol/worker.py"),
            PathBuf::from("../v0/01-worker-protocol/worker.py"),
            PathBuf::from("../../v0/01-worker-protocol/worker.py"),
            PathBuf::from("/usr/share/casette/worker.py"),
        ];

        for rel in &candidates {
            if rel.exists() {
                return Ok(rel.canonicalize().unwrap_or_else(|_| rel.clone()));
            }
        }

        // Try checking relative to current executable
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidate = dir.join("worker.py");
                if candidate.exists() {
                    return Ok(candidate);
                }
                let share_candidate = dir.join("../share/casette/worker.py");
                if share_candidate.exists() {
                    return Ok(share_candidate.canonicalize().unwrap_or(share_candidate));
                }
            }
        }

        Err(KernelError::DiscoveryFailed(
            "could not locate 'worker.py'. Set CASETTE_WORKER_SCRIPT or run from the repository root.".into()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_locate_worker_script() {
        let path = SageDiscovery::locate_worker_script(None).expect("locate worker.py");
        assert!(path.exists());
        assert!(path.ends_with("worker.py"));
    }

    #[tokio::test]
    async fn test_discover_sage() {
        let env = SageDiscovery::discover()
            .await
            .expect("discover sage environment");
        assert!(env.python_path.exists() || env.python_path == Path::new("python3"));
        assert!(!env.sage_version.is_empty());
        assert!(env.worker_script.exists());
    }
}
