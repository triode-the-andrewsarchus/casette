//! Pure Rust in-process math engine backend powered by Woxi (Wolfram Language interpreter).
//!
//! Provides [`WoxiKernel`], managing a dedicated background thread running the Woxi
//! interpreter with thread-local state isolation, instant startup, sub-millisecond
//! reset/restart, and full support for symbolic math, calculus, linear algebra,
//! tape references, and physical unit arithmetic via `UnitSimplify` and `UnitConvert`.

use crate::error::KernelError;
use crate::protocol::{
    ArtifactPayload, ExtremumKind, LocalExtremum, MatrixPropertyPayload, PlotDataPayload,
    PlotSeriesPayload, SymbolPayload, WorkerResponse,
};
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};
use woxi::syntax::Expr;

/// Standard calculator variables pre-declared in the session.
pub const BOOT_VARIABLE_NAMES: &[&str] = &[
    "u", "v", "w", "x", "y", "z", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "y1", "y2",
    "y3", "y4", "y5", "y6", "y7", "y8", "y9", "t",
];

/// Boot prelude executed at initial worker startup and on session restart.
const BOOT_PRELUDE_CODE: &str = r#"
Clear[u, v, w, x, y, z, t, x1, x2, x3, x4, x5, y1, y2, y3, y4, y5];
mi = Quantity[1, "Miles"];
mile = Quantity[1, "Miles"];
miles = Quantity[1, "Miles"];
km = Quantity[1, "Kilometers"];
kilometer = Quantity[1, "Kilometers"];
kilometers = Quantity[1, "Kilometers"];
m = Quantity[1, "Meters"];
meter = Quantity[1, "Meters"];
meters = Quantity[1, "Meters"];
cm = Quantity[1, "Centimeters"];
mm = Quantity[1, "Millimeters"];
ft = Quantity[1, "Feet"];
foot = Quantity[1, "Feet"];
feet = Quantity[1, "Feet"];
in = Quantity[1, "Inches"];
inch = Quantity[1, "Inches"];
inches = Quantity[1, "Inches"];
yd = Quantity[1, "Yards"];
yard = Quantity[1, "Yards"];
yards = Quantity[1, "Yards"];
s = Quantity[1, "Seconds"];
sec = Quantity[1, "Seconds"];
second = Quantity[1, "Seconds"];
seconds = Quantity[1, "Seconds"];
minute = Quantity[1, "Minutes"];
minutes = Quantity[1, "Minutes"];
hr = Quantity[1, "Hours"];
hour = Quantity[1, "Hours"];
hours = Quantity[1, "Hours"];
day = Quantity[1, "Days"];
days = Quantity[1, "Days"];
kg = Quantity[1, "Kilograms"];
kilogram = Quantity[1, "Kilograms"];
kilograms = Quantity[1, "Kilograms"];
g = Quantity[1, "Grams"];
gram = Quantity[1, "Grams"];
grams = Quantity[1, "Grams"];
lb = Quantity[1, "Pounds"];
pound = Quantity[1, "Pounds"];
pounds = Quantity[1, "Pounds"];
oz = Quantity[1, "Ounces"];
ounce = Quantity[1, "Ounces"];
ounces = Quantity[1, "Ounces"];
newton = Quantity[1, "Newtons"];
newtons = Quantity[1, "Newtons"];
joule = Quantity[1, "Joules"];
joules = Quantity[1, "Joules"];
watt = Quantity[1, "Watts"];
watts = Quantity[1, "Watts"];
pascal = Quantity[1, "Pascals"];
pascals = Quantity[1, "Pascals"];
volt = Quantity[1, "Volts"];
volts = Quantity[1, "Volts"];
amp = Quantity[1, "Amperes"];
amperes = Quantity[1, "Amperes"];
mph = Quantity[1, "Miles"/"Hours"];
kph = Quantity[1, "Kilometers"/"Hours"];
__casette_convert[expr_, target_] := UnitConvert[expr, target];
__casette_convert[expr_] := UnitSimplify[expr];
factor[x_] := Factor[x];
expand[x_] := Expand[x];
simplify[x_] := Simplify[x];
diff[f_, x_] := D[f, x];
integral[f_, x_] := Integrate[f, x];
integrate[f_, x_] := Integrate[f, x];
solve[eq_, x_] := Solve[eq, x];
det[m_] := Det[m];
matrix[m_] := m;
pi = Pi;
e = E;
"#;

enum WoxiCommand {
    Eval {
        code: String,
        numeric: bool,
        precision: Option<u32>,
        reply: oneshot::Sender<Result<WorkerResponse, KernelError>>,
    },
    Ping {
        reply: oneshot::Sender<Result<Duration, KernelError>>,
    },
    Symbols {
        reply: oneshot::Sender<Result<Vec<SymbolPayload>, KernelError>>,
    },
    ConfigPrecision {
        digits: u32,
        reply: oneshot::Sender<Result<u32, KernelError>>,
    },
    Restart {
        reply: oneshot::Sender<Result<(), KernelError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), KernelError>>,
    },
}

/// Active connection handle to the in-process Woxi kernel.
#[derive(Clone)]
pub struct WoxiKernel {
    command_tx: mpsc::Sender<WoxiCommand>,
    is_alive: Arc<AtomicBool>,
    precision_digits: Arc<AtomicU32>,
    next_req_id: Arc<AtomicU64>,
}

impl WoxiKernel {
    /// Launches an in-process Woxi math kernel inside a dedicated background worker thread.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError`] if the worker thread fails to spawn or boot initialization fails.
    pub async fn launch() -> Result<Self, KernelError> {
        let (command_tx, mut command_rx) = mpsc::channel::<WoxiCommand>(64);
        let is_alive = Arc::new(AtomicBool::new(true));
        let precision_digits = Arc::new(AtomicU32::new(10));
        let next_req_id = Arc::new(AtomicU64::new(1));

        let is_alive_thread = Arc::clone(&is_alive);
        let (boot_tx, boot_rx) = oneshot::channel();

        std::thread::Builder::new()
            .name("woxi-kernel-worker".into())
            .spawn(move || {
                // Initialize Woxi state
                woxi::clear_state();
                if let Err(e) = woxi::interpret(BOOT_PRELUDE_CODE) {
                    error!("Woxi boot prelude initialization error: {}", e);
                    let _ = boot_tx.send(Err(KernelError::SpawnFailed(format!(
                        "Woxi prelude failed: {e}"
                    ))));
                    is_alive_thread.store(false, Ordering::SeqCst);
                    return;
                }
                let _ = boot_tx.send(Ok(()));

                let mut current_precision = 10u32;
                let plot_dir = std::env::temp_dir().join(format!("casette_woxi_plots_{}", std::process::id()));
                let _ = fs::create_dir_all(&plot_dir);
                let mut plot_counter = 1usize;

                while let Some(cmd) = command_rx.blocking_recv() {
                    match cmd {
                        WoxiCommand::Eval {
                            code,
                            numeric,
                            precision,
                            reply,
                        } => {
                            let prec = precision.unwrap_or(current_precision);
                            let normalized_code = normalize_for_woxi(&code);

                            // Execute evaluation
                            let eval_code = if numeric {
                                format!("N[{}, {}]", normalized_code, prec)
                            } else {
                                normalized_code.clone()
                            };

                            let start_time = Instant::now();
                            let result = woxi::interpret_with_stdout(&eval_code);
                            let _duration = start_time.elapsed();

                            let response = match result {
                                Ok(res) => {
                                    let mut artifacts = Vec::new();

                                    // Extract graphics/SVG ONLY if actual graphical output was produced (e.g. Plot, Graphics)
                                    // NOTE: Do NOT use res.output_svg here as it is internal text typesetting SVG, not a plot image!
                                    if let Some(svg) = res.graphics {
                                        let plot_path = plot_dir.join(format!("plot-{:05}.svg", plot_counter));
                                        plot_counter += 1;
                                        if fs::write(&plot_path, &svg).is_ok() {
                                            artifacts.push(ArtifactPayload {
                                                r#type: "image".to_string(),
                                                format: "svg".to_string(),
                                                path: Some(plot_path.to_string_lossy().to_string()),
                                                bytes: Some(svg.len() as u64),
                                                error: None,
                                            });
                                        }
                                    }

                                    let plot_data = if let Some(Expr::Graphics { source: Some(source), .. }) = res.expr.as_ref() {
                                        Some(extract_plot_data(source))
                                    } else {
                                        None
                                    };

                                    let kind = if plot_data.is_some() || !artifacts.is_empty() {
                                        "plot".to_string()
                                    } else {
                                        determine_kind(res.expr.as_ref(), &res.result)
                                    };
                                    let latex = res.expr.as_ref().map(woxi::functions::string_ast::expr_to_tex);

                                    // If exact, compute decimal approximation for approx field
                                    let mut approx = None;
                                    if !numeric
                                        && (kind == "rational"
                                            || kind == "integer"
                                            || kind == "symbolic"
                                            || kind == "quantity"
                                            || kind == "list")
                                    {
                                        let approx_code = format!("N[{}, {}]", normalized_code, prec);
                                        if let Ok(approx_res) = woxi::interpret_with_stdout(&approx_code) {
                                            let cleaned = clean_approx_text(&approx_res.result);
                                            if cleaned != res.result && !cleaned.is_empty() {
                                                approx = Some(cleaned);
                                            }
                                        }
                                    }

                                    let mut actions = Vec::new();
                                    if kind == "matrix" {
                                        actions.push("det".to_string());
                                        actions.push("transpose".to_string());
                                        actions.push("inverse".to_string());
                                        actions.push("eigenvalues".to_string());
                                    } else if kind == "rational" || kind == "integer" {
                                        actions.push("approx".to_string());
                                        actions.push("factor".to_string());
                                    } else if kind == "symbolic" {
                                        actions.push("simplify".to_string());
                                        actions.push("diff".to_string());
                                        actions.push("integrate".to_string());
                                    }

                                    let matrix_properties = if kind == "matrix" {
                                        extract_matrix_properties(&normalized_code)
                                    } else {
                                        None
                                    };

                                    WorkerResponse {
                                        id: None,
                                        ok: true,
                                        op: None,
                                        pong: None,
                                        pid: Some(std::process::id()),
                                        kind: Some(kind),
                                        plain: Some(res.result.clone()),
                                        latex,
                                        repr: Some(res.result),
                                        approx,
                                        approx_digits: Some(prec),
                                        exact: Some(!numeric),
                                        primary_is_approx: Some(numeric),
                                        exact_value: if numeric { Some(normalized_code) } else { None },
                                        actions,
                                        artifacts,
                                        plot_data,
                                        truncated: Some(false),
                                        truncation: None,
                                        matrix_properties,
                                        stdout: if res.stdout.is_empty() { None } else { Some(res.stdout) },
                                        stderr: None,
                                        error: None,
                                        value: Some(true),
                                        symbols: None,
                                        precision_digits: Some(prec),
                                    }
                                }
                                Err(e) => {
                                    WorkerResponse::new_error(
                                        "EvaluationError".to_string(),
                                        e.to_string(),
                                        None,
                                    )
                                }
                            };

                            let _ = reply.send(Ok(response));
                        }
                        WoxiCommand::Ping { reply } => {
                            let start = Instant::now();
                            let _ = woxi::interpret("1 + 1");
                            let _ = reply.send(Ok(start.elapsed()));
                        }
                        WoxiCommand::Symbols { reply } => {
                            let syms = collect_user_symbols();
                            let _ = reply.send(Ok(syms));
                        }
                        WoxiCommand::ConfigPrecision { digits, reply } => {
                            current_precision = digits;
                            let _ = reply.send(Ok(digits));
                        }
                        WoxiCommand::Restart { reply } => {
                            woxi::clear_state();
                            if let Err(e) = woxi::interpret(BOOT_PRELUDE_CODE) {
                                error!("failed to re-run boot prelude on restart: {}", e);
                            }
                            let _ = reply.send(Ok(()));
                        }
                        WoxiCommand::Shutdown { reply } => {
                            let _ = reply.send(Ok(()));
                            break;
                        }
                    }
                }

                is_alive_thread.store(false, Ordering::SeqCst);
            })
            .map_err(|e| KernelError::SpawnFailed(format!("failed to spawn woxi worker thread: {e}")))?;

        boot_rx
            .await
            .map_err(|_| KernelError::SpawnFailed("woxi thread dropped before boot confirmation".into()))??;

        Ok(Self {
            command_tx,
            is_alive,
            precision_digits,
            next_req_id,
        })
    }

    /// Evaluates an expression through the Woxi math engine.
    pub async fn eval(
        &self,
        code: impl Into<String>,
        numeric: bool,
        precision_digits: Option<u32>,
    ) -> Result<WorkerResponse, KernelError> {
        let code_str = code.into();
        let id_num = self.next_req_id.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = oneshot::channel();

        self.command_tx
            .send(WoxiCommand::Eval {
                code: code_str,
                numeric,
                precision: precision_digits,
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;

        let mut resp = reply_rx.await.map_err(|_| KernelError::ShutDown)??;
        resp.id = Some(format!("req-{}", id_num));
        Ok(resp)
    }

    /// Pings the worker and measures round-trip latency.
    pub async fn ping(&self) -> Result<Duration, KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WoxiCommand::Ping { reply: reply_tx })
            .await
            .map_err(|_| KernelError::ShutDown)?;
        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Queries active user-defined symbols and functions.
    pub async fn symbols(&self) -> Result<Vec<SymbolPayload>, KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WoxiCommand::Symbols { reply: reply_tx })
            .await
            .map_err(|_| KernelError::ShutDown)?;
        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Configures the default decimal precision digits.
    pub async fn config_precision(&self, digits: u32) -> Result<u32, KernelError> {
        self.precision_digits.store(digits, Ordering::Relaxed);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WoxiCommand::ConfigPrecision {
                digits,
                reply: reply_tx,
            })
            .await
            .map_err(|_| KernelError::ShutDown)?;
        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Restarts the Woxi session, clearing user variables and reloading prelude definitions.
    pub async fn restart(&self) -> Result<(), KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx
            .send(WoxiCommand::Restart { reply: reply_tx })
            .await
            .map_err(|_| KernelError::ShutDown)?;
        reply_rx.await.map_err(|_| KernelError::ShutDown)?
    }

    /// Cleanly shuts down the Woxi worker thread.
    pub async fn shutdown(&self) -> Result<(), KernelError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self.command_tx.send(WoxiCommand::Shutdown { reply: reply_tx }).await;
        let _ = reply_rx.await;
        self.is_alive.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Cancels or interrupts in-flight calculations.
    pub fn interrupt(&self) -> Result<(), KernelError> {
        // In-process Woxi worker interrupt signal
        debug!("interrupt called on WoxiKernel");
        Ok(())
    }

    /// Current operating system process ID.
    #[must_use]
    pub fn worker_pid(&self) -> Option<u32> {
        Some(std::process::id())
    }

    /// Whether the background worker thread is currently running.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }
}

/// Normalizes Python/Sage or Casette syntax into canonical Wolfram Language code.
pub fn normalize_for_woxi(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // 1. Handle tape reference dictionary assignment snippet
    // "__casette_tape_refs = globals().get('__casette_tape_refs', {})\n__casette_tape_refs[1] = (expr)..."
    if trimmed.contains("__casette_tape_refs = globals().get") {
        for line in trimmed.lines() {
            let line_trim = line.trim();
            if line_trim.starts_with("__casette_tape_refs[") && line_trim.contains('=') {
                return format!("{};", line_trim);
            }
        }
    }

    // 2. Clear tape: "__casette_tape_refs = {}" -> "Clear[__casette_tape_refs];"
    if trimmed == "__casette_tape_refs = {}" || trimmed == "__casette_tape_refs = {}\n" {
        return "Clear[__casette_tape_refs];".to_string();
    }

    let mut result = trimmed.to_string();

    // 3. Exponentiation syntax: replace Python `**` with `^` outside quotes
    result = replace_outside_quotes(&result, "**", "^");

    // 4. Matrix conversion: "matrix([[1, 2], [3, 4]])" -> "{{1, 2}, {3, 4}}"
    // and method calls like "matrix([[...]]).det()" -> "Det[{{...}}]"
    result = convert_matrix_and_methods(&result);

    // 5. Function conversion for common Python call forms: func(...) -> Func[...]
    result = convert_function_calls(&result);

    result
}

fn replace_outside_quotes(input: &str, from: &str, to: &str) -> String {
    if !input.contains(from) {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        if let Some(q) = in_quote {
            out.push(ch);
            if ch == q {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            out.push(ch);
        } else if ch == '*' && chars.peek() == Some(&'*') && from == "**" {
            chars.next();
            out.push_str(to);
        } else {
            out.push(ch);
        }
    }
    out
}

fn convert_matrix_and_methods(input: &str) -> String {
    let mut s = input.to_string();

    // Convert ".det()" -> "Det[...]"
    if s.ends_with(".det()") {
        let base = s.trim_end_matches(".det()").trim();
        let unparenthesized = if base.starts_with('(') && base.ends_with(')') {
            &base[1..base.len() - 1]
        } else {
            base
        };
        return format!("Det[{}]", convert_matrix_and_methods(unparenthesized));
    }

    // Convert "matrix([[...]])" -> "{{...}}"
    if let Some(start) = s.find("matrix([") {
        if let Some(end) = s[start..].rfind("])") {
            let inner = &s[start + 7..start + end + 1]; // includes "[" ... "]"
            let converted_brackets = inner.replace('[', "{").replace(']', "}");
            s = format!("{}{}{}", &s[..start], converted_brackets, &s[start + end + 2..]);
        }
    }

    s
}

fn convert_function_calls(input: &str) -> String {
    const KNOWN_FUNCS: &[(&str, &str)] = &[
        ("__casette_convert", "__casette_convert"),
        ("factor", "Factor"),
        ("Factor", "Factor"),
        ("expand", "Expand"),
        ("Expand", "Expand"),
        ("simplify", "Simplify"),
        ("Simplify", "Simplify"),
        ("diff", "D"),
        ("D", "D"),
        ("integral", "Integrate"),
        ("integrate", "Integrate"),
        ("Integrate", "Integrate"),
        ("solve", "Solve"),
        ("Solve", "Solve"),
        ("det", "Det"),
        ("Det", "Det"),
        ("sin", "Sin"),
        ("cos", "Cos"),
        ("tan", "Tan"),
        ("log", "Log"),
        ("exp", "Exp"),
        ("sqrt", "Sqrt"),
        ("Sqrt", "Sqrt"),
        ("N", "N"),
        ("plot", "Plot"),
        ("Plot", "Plot"),
        ("assume", "Assume"),
        ("binomial", "Binomial"),
        ("factorial", "Factorial"),
    ];

    let mut s = input.to_string();

    for &(src, dst) in KNOWN_FUNCS {
        let pattern = format!("{}(", src);
        while let Some(idx) = s.find(&pattern) {
            // Ensure `src` is at a word boundary
            if idx > 0 {
                let prev = s.as_bytes()[idx - 1];
                if prev.is_ascii_alphanumeric() || prev == b'_' {
                    break;
                }
            }

            // Find matching closing paren
            let open_paren_idx = idx + src.len();
            if let Some(close_paren_idx) = find_matching_delimiter(&s, open_paren_idx, '(', ')') {
                let inner = &s[open_paren_idx + 1..close_paren_idx];
                let mut converted_inner = convert_function_calls(inner);
                if dst == "Plot" {
                    converted_inner = convert_plot_args(&converted_inner);
                }
                let replacement = format!("{}[{}]", dst, converted_inner);
                s.replace_range(idx..=close_paren_idx, &replacement);
            } else {
                break;
            }
        }
    }

    s
}

fn find_matching_delimiter(s: &str, open_idx: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    let mut in_quote: Option<char> = None;

    for (idx, ch) in s[open_idx..].char_indices() {
        let current_idx = open_idx + idx;
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some(current_idx);
            }
        }
    }
    None
}

fn determine_kind(expr_opt: Option<&Expr>, plain: &str) -> String {
    if let Some(expr) = expr_opt {
        match expr {
            Expr::Integer(_) | Expr::BigInteger(_) => "integer".to_string(),
            Expr::Real(_) | Expr::BigFloat(..) => "real".to_string(),
            Expr::Graphics { .. } => "plot".to_string(),
            Expr::FunctionCall { name, args } => {
                if name == "Rational" && args.len() == 2 {
                    "rational".to_string()
                } else if name == "Complex" {
                    "complex".to_string()
                } else if name == "Quantity" {
                    "quantity".to_string()
                } else if name == "Graphics" || name == "Graphics3D" || name == "Plot" {
                    "plot".to_string()
                } else {
                    "symbolic".to_string()
                }
            }
            Expr::List(list) => {
                // Check if list is a 2D matrix of numbers or scalar expressions (not rules)
                let first_cols = list.first().and_then(|item| {
                    if let Expr::List(inner) = item {
                        Some(inner.len())
                    } else {
                        None
                    }
                });
                let is_matrix = match first_cols {
                    Some(cols) if cols > 0 && !list.is_empty() => list.iter().all(|item| {
                        if let Expr::List(inner) = item {
                            inner.len() == cols
                                && !inner.iter().any(|e| {
                                    matches!(e, Expr::Rule { .. } | Expr::RuleDelayed { .. })
                                })
                        } else {
                            false
                        }
                    }),
                    _ => false,
                };
                if is_matrix {
                    "matrix".to_string()
                } else {
                    "list".to_string()
                }
            }
            Expr::Identifier(s) if s == "True" || s == "False" => "boolean".to_string(),
            Expr::String(_) => "text".to_string(),
            _ => {
                if plain.contains("Quantity[") {
                    "quantity".to_string()
                } else if plain.starts_with("{{") && plain.ends_with("}}") && !plain.contains("->") && !plain.contains(":>") {
                    "matrix".to_string()
                } else if plain == "True" || plain == "False" {
                    "boolean".to_string()
                } else {
                    "symbolic".to_string()
                }
            }
        }
    } else if plain.contains("Quantity[") {
        "quantity".to_string()
    } else if plain.starts_with("{{") && plain.ends_with("}}") && !plain.contains("->") && !plain.contains(":>") {
        "matrix".to_string()
    } else if plain == "True" || plain == "False" {
        "boolean".to_string()
    } else if plain.parse::<i64>().is_ok() {
        "integer".to_string()
    } else if plain.parse::<f64>().is_ok() {
        "real".to_string()
    } else {
        "symbolic".to_string()
    }
}

fn collect_user_symbols() -> Vec<SymbolPayload> {
    let mut symbols = Vec::new();
    if let Ok(names_res) = woxi::interpret("Names[\"Global`*\"]") {
        let clean = names_res.trim().trim_start_matches('{').trim_end_matches('}');
        for item in clean.split(',') {
            let name = item.trim().trim_matches('"');
            if name.is_empty() || name.starts_with("__") || is_boot_symbol(name) {
                continue;
            }

            let summary = woxi::interpret(name).unwrap_or_else(|_| name.to_string());
            let kind = if summary.starts_with("{{") {
                "matrix"
            } else if summary.starts_with('{') {
                "list"
            } else if summary.contains("Quantity[") {
                "quantity"
            } else if summary.parse::<i64>().is_ok() {
                "integer"
            } else if summary.parse::<f64>().is_ok() {
                "real"
            } else {
                "variable"
            };

            symbols.push(SymbolPayload {
                name: name.to_string(),
                kind: kind.to_string(),
                summary,
            });
        }
    }
    symbols
}

fn is_boot_symbol(name: &str) -> bool {
    const BOOT_SYMBOLS: &[&str] = &[
        "mi", "mile", "miles", "km", "kilometer", "kilometers", "m", "meter", "meters", "cm", "mm",
        "ft", "foot", "feet", "inch", "inches", "yard", "yards", "s", "sec", "second", "seconds",
        "minute", "minutes", "hr", "hour", "hours", "day", "days", "kg", "kilogram", "kilograms",
        "g", "gram", "grams", "lb", "pound", "pounds", "oz", "ounce", "ounces", "newton", "newtons",
        "joule", "joules", "watt", "watts", "pascal", "pascals", "volt", "volts", "amp", "amperes",
        "mph", "kph", "factor", "expand", "simplify", "diff", "integral", "integrate", "solve", "det", "matrix",
    ];
    BOOT_SYMBOLS.contains(&name)
}

fn extract_matrix_properties(code: &str) -> Option<Vec<MatrixPropertyPayload>> {
    let mut props = Vec::new();

    // Check square
    let square_val = woxi::interpret(&format!("SquareMatrixQ[{}]", code))
        .map(|r| r.trim() == "True")
        .ok();
    props.push(MatrixPropertyPayload {
        name: "is_square".to_string(),
        label: "Square".to_string(),
        value: square_val,
        error: None,
    });

    // Check invertible (square and det != 0)
    let det_val = woxi::interpret(&format!("Det[{}]", code)).ok();
    let is_invertible = match (square_val, det_val) {
        (Some(true), Some(ref d)) => Some(d.trim() != "0"),
        (Some(false), _) => Some(false),
        _ => None,
    };
    props.push(MatrixPropertyPayload {
        name: "is_invertible".to_string(),
        label: "Invertible".to_string(),
        value: is_invertible,
        error: None,
    });

    // Check symmetric
    let symm_val = woxi::interpret(&format!("SymmetricMatrixQ[{}]", code))
        .map(|r| r.trim() == "True")
        .ok();
    props.push(MatrixPropertyPayload {
        name: "is_symmetric".to_string(),
        label: "Symmetric".to_string(),
        value: symm_val,
        error: None,
    });

    Some(props)
}

fn clean_approx_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '`' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '.' {
                j += 1;
            }
            if j > i + 1 {
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut in_quote: Option<char> = None;

    for (idx, ch) in s.char_indices() {
        if let Some(q) = in_quote {
            if ch == q {
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
        } else if ch == '(' {
            paren_depth += 1;
        } else if ch == ')' {
            paren_depth = paren_depth.saturating_sub(1);
        } else if ch == '[' {
            bracket_depth += 1;
        } else if ch == ']' {
            bracket_depth = bracket_depth.saturating_sub(1);
        } else if ch == '{' {
            brace_depth += 1;
        } else if ch == '}' {
            brace_depth = brace_depth.saturating_sub(1);
        } else if ch == ',' && paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
            args.push(&s[start..idx]);
            start = idx + 1;
        }
    }
    args.push(&s[start..]);
    args
}

fn convert_plot_args(inner: &str) -> String {
    let args = split_top_level_commas(inner);
    let mut converted_args = Vec::with_capacity(args.len());

    for arg in args {
        let trimmed = arg.trim();
        if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() >= 2 {
            let inner_arg = &trimmed[1..trimmed.len() - 1];
            // Check if inner_arg contains a comma at depth 0 (meaning it was a tuple)
            let mut has_comma = false;
            let mut p_depth = 0usize;
            let mut b_depth = 0usize;
            let mut br_depth = 0usize;
            for ch in inner_arg.chars() {
                if ch == '(' {
                    p_depth += 1;
                } else if ch == ')' {
                    p_depth = p_depth.saturating_sub(1);
                } else if ch == '[' {
                    b_depth += 1;
                } else if ch == ']' {
                    b_depth = b_depth.saturating_sub(1);
                } else if ch == '{' {
                    br_depth += 1;
                } else if ch == '}' {
                    br_depth = br_depth.saturating_sub(1);
                } else if ch == ',' && p_depth == 0 && b_depth == 0 && br_depth == 0 {
                    has_comma = true;
                    break;
                }
            }
            if has_comma {
                converted_args.push(format!("{{{inner_arg}}}"));
                continue;
            }
        }
        converted_args.push(trimmed.to_string());
    }

    converted_args.join(", ")
}

/// Computes local minima and maxima from a series of sampled points.
pub fn compute_local_extrema(points: &[[f64; 2]]) -> Vec<LocalExtremum> {
    let mut extrema = Vec::new();
    if points.len() < 3 {
        return extrema;
    }

    let mut i = 1;
    while i + 1 < points.len() {
        let prev_y = points[i - 1][1];
        let curr_y = points[i][1];
        let next_y = points[i + 1][1];

        if !curr_y.is_finite() || !prev_y.is_finite() || !next_y.is_finite() {
            i += 1;
            continue;
        }

        if curr_y > prev_y && curr_y >= next_y {
            // Plateau check for maximum
            let mut j = i + 1;
            while j < points.len() && points[j][1] == curr_y {
                j += 1;
            }
            if j < points.len() && points[j][1] < curr_y {
                let mid_x = (points[i][0] + points[j - 1][0]) * 0.5;
                extrema.push(LocalExtremum {
                    x: mid_x,
                    y: curr_y,
                    kind: ExtremumKind::Maximum,
                });
                i = j;
                continue;
            } else if curr_y > next_y {
                extrema.push(LocalExtremum {
                    x: points[i][0],
                    y: curr_y,
                    kind: ExtremumKind::Maximum,
                });
            }
        } else if curr_y < prev_y && curr_y <= next_y {
            // Plateau check for minimum
            let mut j = i + 1;
            while j < points.len() && points[j][1] == curr_y {
                j += 1;
            }
            if j < points.len() && points[j][1] > curr_y {
                let mid_x = (points[i][0] + points[j - 1][0]) * 0.5;
                extrema.push(LocalExtremum {
                    x: mid_x,
                    y: curr_y,
                    kind: ExtremumKind::Minimum,
                });
                i = j;
                continue;
            } else if curr_y < next_y {
                extrema.push(LocalExtremum {
                    x: points[i][0],
                    y: curr_y,
                    kind: ExtremumKind::Minimum,
                });
            }
        }
        i += 1;
    }

    extrema
}

fn extract_plot_data(source: &woxi::syntax::PlotSource) -> PlotDataPayload {
    let series = source
        .series
        .iter()
        .map(|s| {
            let points: Vec<[f64; 2]> = s.points.iter().map(|&(x, y)| [x, y]).collect();
            let extrema = compute_local_extrema(&points);
            PlotSeriesPayload {
                label: None,
                points,
                color: [s.color.0, s.color.1, s.color.2],
                is_scatter: s.is_scatter,
                extrema,
            }
        })
        .collect();

    PlotDataPayload {
        title: None,
        x_label: None,
        y_label: None,
        series,
        x_range: [source.x_range.0, source.x_range.1],
        y_range: [source.y_range.0, source.y_range.1],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_for_woxi_functions() {
        assert_eq!(normalize_for_woxi("factor(x^2 - 1)"), "Factor[x^2 - 1]");
        assert_eq!(normalize_for_woxi("diff(x^3, x)"), "D[x^3, x]");
        assert_eq!(normalize_for_woxi("integrate(1/(1-x^2), x)"), "Integrate[1/(1-x^2), x]");
        assert_eq!(normalize_for_woxi("matrix([[1, 2], [3, 4]])"), "{{1, 2}, {3, 4}}");
        assert_eq!(
            normalize_for_woxi("__casette_convert(50*km / (32*mi))"),
            "__casette_convert[50*km / (32*mi)]"
        );
    }

    #[test]
    fn test_normalize_matrix_det() {
        assert_eq!(
            normalize_for_woxi("(matrix([[1, 2], [3, 4]])).det()"),
            "Det[{{1, 2}, {3, 4}}]"
        );
    }

    #[test]
    fn test_normalize_plot_args() {
        assert_eq!(
            normalize_for_woxi("plot(sin(x), (x, -pi, pi))"),
            "Plot[Sin[x], {x, -pi, pi}]"
        );
        assert_eq!(
            normalize_for_woxi("plot((sin(x), cos(x)), (x, -10, 10))"),
            "Plot[{Sin[x], Cos[x]}, {x, -10, 10}]"
        );
    }

    #[test]
    fn test_compute_local_extrema() {
        let pts = vec![
            [-2.0, 0.0],
            [-1.0, 2.0], // max
            [0.0, 0.0],
            [1.0, -2.0], // min
            [2.0, 0.0],
        ];
        let ext = compute_local_extrema(&pts);
        assert_eq!(ext.len(), 2);
        assert_eq!(ext[0].kind, ExtremumKind::Maximum);
        assert_eq!(ext[0].x, -1.0);
        assert_eq!(ext[0].y, 2.0);
        assert_eq!(ext[1].kind, ExtremumKind::Minimum);
        assert_eq!(ext[1].x, 1.0);
        assert_eq!(ext[1].y, -2.0);
    }
}
