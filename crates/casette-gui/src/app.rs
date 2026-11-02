//! Main eframe application coordinating GUI state and background kernel workers.

use crate::model::{AsyncMessage, GuiAction, SessionState, WorkerStatus};
use crate::render::TextureCache;
use crate::theme::{apply_theme, Colors};
use crate::views::{render_header, render_input_pane, render_sidebar, render_tape, SidebarTab};
use eframe::App;
use egui::{
    Button, CentralPanel, Context, FontFamily, FontId, RichText, SidePanel, Stroke, TopBottomPanel,
    Window,
};
use std::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

/// Main Casette GUI application structure.
pub struct CasetteApp {
    state: SessionState,
    session_path: std::path::PathBuf,
    texture_cache: TextureCache,
    action_tx: Sender<GuiAction>,
    message_rx: Receiver<AsyncMessage>,
    sidebar_visible: bool,
    current_sidebar_tab: SidebarTab,
    symbol_filter: String,
    show_builtin_symbols: bool,
    doctor_open: bool,
    help_open: bool,
    last_md_export: Option<std::time::Instant>,
}

impl CasetteApp {
    /// Creates a new Casette desktop application instance.
    #[must_use]
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        action_tx: Sender<GuiAction>,
        message_rx: Receiver<AsyncMessage>,
    ) -> Self {
        apply_theme(&_cc.egui_ctx);

        let session_path = crate::persistence::default_session_path();
        let mut state = SessionState::default();
        if session_path.exists() {
            if let Err(e) = crate::persistence::load_session(&mut state, &session_path) {
                tracing::warn!(
                    "Failed to load previous session from {:?}: {}",
                    session_path,
                    e
                );
            } else {
                tracing::info!(
                    "Loaded previous session: {} tape rows, {} history items",
                    state.rows.len(),
                    state.history.len()
                );
            }
        }

        Self {
            state,
            session_path,
            texture_cache: TextureCache::new(),
            action_tx,
            message_rx,
            sidebar_visible: true,
            current_sidebar_tab: SidebarTab::Symbols,
            symbol_filter: String::new(),
            show_builtin_symbols: false,
            doctor_open: false,
            help_open: false,
            last_md_export: None,
        }
    }

    fn poll_async_messages(&mut self, ctx: &Context) {
        while let Ok(msg) = self.message_rx.try_recv() {
            match msg {
                AsyncMessage::WorkerReady { pid, version } => {
                    info!("GUI received WorkerReady: PID={}, version={}", pid, version);
                    self.state.worker_pid = Some(pid);
                    self.state.sage_version = version;
                    self.state.worker_status = WorkerStatus::Ready;

                    let _ = self
                        .action_tx
                        .send(GuiAction::SetPrecision(self.state.precision_digits));

                    if !self.state.user_definitions.is_empty() {
                        let _ = self.action_tx.send(GuiAction::SyncDefinitions(
                            self.state.user_definitions.clone(),
                        ));
                    }

                    let _ = self.action_tx.send(GuiAction::FetchSymbols);

                    let valid_refs: Vec<(usize, String)> = self
                        .state
                        .rows
                        .iter()
                        .filter_map(|r| r.reusable_expression().map(|expr| (r.row_number, expr)))
                        .collect();
                    if !valid_refs.is_empty() {
                        let _ = self.action_tx.send(GuiAction::SyncTapeRefs(valid_refs));
                    }
                }
                AsyncMessage::EvalFinished {
                    row_number,
                    response,
                    duration,
                } => {
                    let mut is_success = false;
                    let mut input_raw = String::new();
                    let mut compiled_sage = String::new();

                    if let Some(row) = self
                        .state
                        .rows
                        .iter_mut()
                        .find(|r| r.row_number == row_number)
                    {
                        row.complete(*response, duration);
                        is_success = row.result.as_ref().map(|r| r.ok).unwrap_or(false);
                        input_raw = row.input_raw.clone();
                        compiled_sage = row.compiled_sage.clone();
                    }

                    if is_success {
                        let trimmed = input_raw.trim();
                        if let Some(rest) = trimmed.strip_prefix("del ") {
                            for part in rest.split(',') {
                                let sym_to_del = part.trim();
                                if !sym_to_del.is_empty() {
                                    self.state.remove_definition(sym_to_del);
                                }
                            }
                        } else {
                            let new_defs =
                                crate::model::extract_definitions(&input_raw, &compiled_sage);
                            for def in new_defs {
                                self.state.set_definition(def.name, def.statement);
                            }
                        }
                    }

                    self.state.worker_status = WorkerStatus::Ready;
                    let _ = self.action_tx.send(GuiAction::FetchSymbols);
                    if let Err(e) =
                        crate::persistence::save_session(&self.state, &self.session_path)
                    {
                        tracing::warn!("Failed to auto-save session: {}", e);
                    }
                }
                AsyncMessage::SymbolsUpdated(symbols) => {
                    self.state.symbols = symbols;
                }
                AsyncMessage::PrecisionUpdated(digits) => {
                    self.state.precision_digits = digits;
                    if let Err(e) =
                        crate::persistence::save_session(&self.state, &self.session_path)
                    {
                        tracing::warn!("Failed to auto-save session after precision update: {}", e);
                    }
                }
                AsyncMessage::StatusChanged(status) => {
                    self.state.worker_status = status;
                }
                AsyncMessage::ErrorOccurred(err) => {
                    error!("kernel error: {}", err);
                    self.state.worker_status = WorkerStatus::Error;
                }
            }
            ctx.request_repaint();
        }
    }

    fn submit_evaluation(&mut self, input: String, numeric: bool) {
        let refs = self.state.tape_references();
        let (compiled, required_vars, compile_err) = if input.contains('\n') {
            (input.clone(), Vec::new(), None)
        } else {
            match friendly_compiler::compile_with_refs(&input, &refs) {
                friendly_compiler::CompileResult::Success {
                    generated_sage,
                    required_variables,
                } => (generated_sage, required_variables, None),
                friendly_compiler::CompileResult::Bypass { raw_sage } => {
                    (raw_sage, Vec::new(), None)
                }
                friendly_compiler::CompileResult::Error(err) => {
                    (input.clone(), Vec::new(), Some(err))
                }
                friendly_compiler::CompileResult::Ambiguous { candidates } => {
                    let msg = format!(
                        "Ambiguous expression. Did you mean:\n{}",
                        candidates.join("\n")
                    );
                    (
                        input.clone(),
                        Vec::new(),
                        Some(friendly_compiler::CompileError::new(msg, None, None)),
                    )
                }
            }
        };

        let row_num = self.state.push_row(input.clone(), compiled.clone());

        if let Some(err) = compile_err {
            let err_resp = casette_kernel::WorkerResponse::new_error(
                "CompileError".into(),
                err.message,
                err.suggestion,
            );
            if let Some(row) = self.state.rows.iter_mut().find(|r| r.row_number == row_num) {
                row.complete(err_resp, std::time::Duration::from_millis(0));
            }
            self.state.worker_status = WorkerStatus::Ready;
            if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
            return;
        }

        self.state.worker_status = WorkerStatus::Busy;

        let non_boot_vars: Vec<String> = required_vars
            .into_iter()
            .filter(|v| !casette_kernel::BOOT_VARIABLE_NAMES.contains(&v.as_str()))
            .collect();
        let eval_code = if non_boot_vars.is_empty() {
            compiled
        } else {
            let quoted = non_boot_vars
                .iter()
                .map(|v| format!("'{v}'"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("_casette_ensure_vars([{quoted}])\n{compiled}")
        };

        let _ = self.action_tx.send(GuiAction::Evaluate {
            row_number: row_num,
            input,
            compiled_code: eval_code,
            numeric,
        });
    }
}

impl App for CasetteApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_async_messages(ctx);

        // 1. Top Panel: Header Toolbar
        TopBottomPanel::top("top_header_panel").show(ctx, |ui| {
            let mut prec_change = None;
            let mut clear_tape = false;
            let mut export_md = false;
            let md_exported_recently = self
                .last_md_export
                .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(2500));

            render_header(
                ui,
                &mut self.state,
                &mut self.sidebar_visible,
                &mut self.doctor_open,
                &mut self.help_open,
                md_exported_recently,
                &mut |digits| prec_change = Some(digits),
                &mut || clear_tape = true,
                &mut || export_md = true,
            );

            if let Some(digits) = prec_change {
                let _ = self.action_tx.send(GuiAction::SetPrecision(digits));
            }
            if clear_tape {
                self.state.clear_tape();
                let _ = self.action_tx.send(GuiAction::ClearTape);
                if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
                    tracing::warn!("Failed to auto-save session after clear tape: {}", e);
                }
            }
            if export_md {
                let md = crate::persistence::export_to_markdown(&self.state);
                ctx.copy_text(md.clone());
                let md_path = self.session_path.with_extension("md");
                if let Err(e) = std::fs::write(&md_path, &md) {
                    tracing::warn!("Failed to write Markdown export to {:?}: {}", md_path, e);
                } else {
                    info!("Exported session Markdown to {:?}", md_path);
                }
                self.last_md_export = Some(std::time::Instant::now());
            }
        });

        // 2. Bottom Panel: Input Editor & Lowering Preview
        TopBottomPanel::bottom("bottom_input_panel").show(ctx, |ui| {
            let mut to_submit = None;
            let mut do_interrupt = false;

            render_input_pane(
                ui,
                &mut self.state,
                &mut |code, numeric| to_submit = Some((code, numeric)),
                &mut || do_interrupt = true,
            );

            if let Some((code, numeric)) = to_submit {
                self.submit_evaluation(code, numeric);
            }
            if do_interrupt {
                let _ = self.action_tx.send(GuiAction::Interrupt);
            }
        });

        // 3. Right Panel: Sidebar (Symbols / History / Inspector / Doctor)
        if self.sidebar_visible {
            SidePanel::right("right_sidebar_panel")
                .min_width(260.0)
                .default_width(280.0)
                .show(ctx, |ui| {
                    let mut refresh_syms = false;
                    let mut restart_worker = false;
                    let mut draft_to_load = None;
                    let mut delete_sym = None;

                    render_sidebar(
                        ui,
                        &mut self.state,
                        &mut self.current_sidebar_tab,
                        &mut self.symbol_filter,
                        &mut self.show_builtin_symbols,
                        &mut || refresh_syms = true,
                        &mut || restart_worker = true,
                        &mut |s| draft_to_load = Some(s),
                        &mut |name| delete_sym = Some(name),
                    );

                    if let Some(sym_name) = delete_sym {
                        self.state.remove_definition(&sym_name);
                        self.state.symbols.retain(|s| s.name != sym_name);
                        let _ = self.action_tx.send(GuiAction::DeleteSymbol(sym_name));
                        if let Err(e) =
                            crate::persistence::save_session(&self.state, &self.session_path)
                        {
                            tracing::warn!(
                                "Failed to auto-save session after symbol deletion: {}",
                                e
                            );
                        }
                    }

                    if refresh_syms {
                        let _ = self.action_tx.send(GuiAction::FetchSymbols);
                    }
                    if restart_worker {
                        let _ = self.action_tx.send(GuiAction::RestartWorker);
                    }
                    if let Some(text) = draft_to_load {
                        self.state.draft_input = text;
                        self.state.request_input_focus = true;
                    }
                });
        }

        // 4. Central Panel: Session Tape
        CentralPanel::default().show(ctx, |ui| {
            let mut action_to_submit: Option<(String, bool)> = None;

            render_tape(
                ui,
                &mut self.state,
                &mut self.texture_cache,
                &mut |action, snippet| {
                    let is_numeric = action == "approx";
                    action_to_submit = Some((snippet.to_string(), is_numeric));
                },
            );

            if let Some((snippet, is_numeric)) = action_to_submit {
                self.submit_evaluation(snippet, is_numeric);
            }
        });

        // 5. Sage Doctor Diagnostics Window (when doctor_open is active)
        if self.doctor_open {
            let mut restart = false;
            let mut open_sidebar_tab = false;

            Window::new("🩺 Sage Doctor")
                .open(&mut self.doctor_open)
                .resizable(false)
                .collapsible(false)
                .default_width(340.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("SageMath Environment Diagnostics")
                            .color(Colors::TEXT_PRIMARY)
                            .font(FontId::new(14.0, FontFamily::Proportional))
                            .strong(),
                    );
                    ui.add_space(8.0);

                    // Version
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Sage Version:")
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(12.0, FontFamily::Proportional)),
                        );
                        ui.label(
                            RichText::new(if self.state.sage_version.is_empty() {
                                "Detecting / Not running"
                            } else {
                                &self.state.sage_version
                            })
                            .color(Colors::TEXT_PRIMARY)
                            .font(FontId::new(12.0, FontFamily::Monospace)),
                        );
                    });

                    // Worker PID
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Worker PID:")
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(12.0, FontFamily::Proportional)),
                        );
                        ui.label(
                            RichText::new(
                                self.state
                                    .worker_pid
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "None".to_string()),
                            )
                            .color(Colors::TEXT_PRIMARY)
                            .font(FontId::new(12.0, FontFamily::Monospace)),
                        );
                    });

                    // Worker Status
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Worker Status:")
                                .color(Colors::TEXT_MUTED)
                                .font(FontId::new(12.0, FontFamily::Proportional)),
                        );
                        let (status_text, status_color) = match self.state.worker_status {
                            WorkerStatus::Ready => ("● Ready", Colors::EXACT_GREEN),
                            WorkerStatus::Starting => ("◌ Starting", Colors::NUMERIC_AMBER),
                            WorkerStatus::Busy => ("◐ Busy", Colors::ACCENT_PRIMARY),
                            WorkerStatus::Restarting => ("◌ Restarting", Colors::NUMERIC_AMBER),
                            WorkerStatus::Error => ("✕ Disconnected", Colors::ERROR_RED),
                        };
                        ui.label(
                            RichText::new(status_text)
                                .color(status_color)
                                .font(FontId::new(12.0, FontFamily::Proportional))
                                .strong(),
                        );
                    });

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new("Restart Sage Worker")
                                        .color(Colors::TEXT_PRIMARY)
                                        .font(FontId::new(12.0, FontFamily::Proportional)),
                                )
                                .fill(Colors::CARD_BG)
                                .stroke(Stroke::new(1.0_f32, Colors::ACCENT_PRIMARY))
                                .corner_radius(6),
                            )
                            .clicked()
                        {
                            restart = true;
                        }

                        if ui
                            .add(
                                Button::new(
                                    RichText::new("Show in Sidebar")
                                        .color(Colors::TEXT_SECONDARY)
                                        .font(FontId::new(12.0, FontFamily::Proportional)),
                                )
                                .corner_radius(6),
                            )
                            .clicked()
                        {
                            open_sidebar_tab = true;
                        }
                    });
                });

            if restart {
                let _ = self.action_tx.send(GuiAction::RestartWorker);
            }
            if open_sidebar_tab {
                self.sidebar_visible = true;
                self.current_sidebar_tab = SidebarTab::Doctor;
                self.doctor_open = false;
            }
        }

        // 6. Reference & Help Window (when help_open is active)
        if self.help_open {
            let mut example_to_load: Option<String> = None;

            Window::new("❓ Casette Reference & Help")
                .open(&mut self.help_open)
                .resizable(true)
                .collapsible(false)
                .default_width(540.0)
                .default_height(460.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(
                            "Click any example below to load it directly into the input bar.",
                        )
                        .color(Colors::TEXT_SECONDARY)
                        .font(FontId::new(12.0, FontFamily::Proportional)),
                    );
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let sections: &[(&str, &[(&str, &str)])] = &[
                                (
                                    "Algebra & Calculus",
                                    &[
                                        ("factor x^4 - 1", "Factor polynomial or integer"),
                                        ("expand (x + y)^3", "Expand symbolic product"),
                                        ("simplify (x^2 - 1)/(x - 1)", "Simplify expression"),
                                        ("solve x^2 + 5*x + 6 = 0 for x", "Solve equation(s)"),
                                        ("derivative sin(x)^2 wrt x", "Symbolic derivative"),
                                        (
                                            "integral x * exp(-x^2), x=0..infinity",
                                            "Definite or indefinite integral",
                                        ),
                                        ("limit sin(x)/x, x->0", "Symbolic limit (supports + / -)"),
                                        (
                                            "taylor cos(x), x=0, order=6",
                                            "Taylor / Maclaurin series",
                                        ),
                                        ("sum 1/k^2, k=1..infinity", "Symbolic summation"),
                                    ],
                                ),
                                (
                                    "Linear Algebra & Vector Calculus",
                                    &[
                                        ("[1 2; 3 4]", "MATLAB-style matrix literal"),
                                        ("det [1, 2; 3, 4]", "Determinant of a matrix"),
                                        ("rref [1, 2, 3; 4, 5, 6]", "Reduced row echelon form"),
                                        ("eigenvalues [2, 1; 1, 2]", "Matrix eigenvalues"),
                                        ("inverse [1, 2; 3, 5]", "Matrix inverse"),
                                        (
                                            "gradient x^2 + x*y^3, [x, y]",
                                            "Symbolic gradient vector",
                                        ),
                                        ("hessian x^3 + y^3 - 3*x*y", "Hessian matrix"),
                                    ],
                                ),
                                (
                                    "Interactive Plotting",
                                    &[
                                        (
                                            "plot sin(x) * cos(2*x), x=-pi..pi",
                                            "2D Cartesian function plot",
                                        ),
                                        (
                                            "implicit_plot x^2 + y^2 = 4, x=-3..3, y=-3..3",
                                            "2D implicit curve plot",
                                        ),
                                        (
                                            "parametric_plot (cos(3*t), sin(2*t)), t=0..2*pi",
                                            "2D parametric curve plot",
                                        ),
                                    ],
                                ),
                                (
                                    "Physical Units & Conversions",
                                    &[
                                        ("32 mi + 5 km to miles", "Length addition and conversion"),
                                        ("12V / 3A to ohm", "Ohm's law electrical unit conversion"),
                                        ("100 degC in degF", "Affine temperature conversion"),
                                        ("50 N * 10 m to J", "Mechanical work to Joules"),
                                    ],
                                ),
                                (
                                    "Probability & Statistics",
                                    &[
                                        (
                                            "normal_cdf 1.96, mean=0, sd=1",
                                            "Normal cumulative distribution",
                                        ),
                                        (
                                            "binomial_pmf 3, n=10, p=0.5",
                                            "Binomial probability mass",
                                        ),
                                        ("mean [4, 8, 15, 16, 23, 42]", "Sample mean"),
                                    ],
                                ),
                                (
                                    "Session Tape References",
                                    &[
                                        ("derivative #1 wrt x", "Reference output of row #1"),
                                        ("subs #1, x=2", "Substitute into output of row #1"),
                                    ],
                                ),
                            ];

                            for (title, items) in sections {
                                ui.label(
                                    RichText::new(*title)
                                        .color(Colors::ACCENT_PRIMARY)
                                        .font(FontId::new(13.0, FontFamily::Proportional))
                                        .strong(),
                                );
                                ui.add_space(3.0);

                                for (example, desc) in *items {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add(
                                                Button::new(
                                                    RichText::new(*example)
                                                        .color(Colors::TEXT_PRIMARY)
                                                        .font(FontId::new(
                                                            11.5,
                                                            FontFamily::Monospace,
                                                        )),
                                                )
                                                .fill(Colors::INPUT_BG)
                                                .corner_radius(4),
                                            )
                                            .on_hover_text("Click to insert into input editor")
                                            .clicked()
                                        {
                                            example_to_load = Some((*example).to_string());
                                        }
                                        ui.label(
                                            RichText::new(*desc)
                                                .color(Colors::TEXT_MUTED)
                                                .font(FontId::new(11.5, FontFamily::Proportional)),
                                        );
                                    });
                                }
                                ui.add_space(10.0);
                            }

                            ui.separator();
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Keyboard Shortcuts")
                                    .color(Colors::ACCENT_PRIMARY)
                                    .font(FontId::new(13.0, FontFamily::Proportional))
                                    .strong(),
                            );
                            for (key, desc) in [
                                ("Enter", "Evaluate current input"),
                                ("Shift + Enter", "Insert newline (multi-line raw Sage)"),
                                ("Tab", "Autocomplete command or session symbol"),
                                ("Up / Down (or Ctrl+Up/Down)", "Navigate evaluation history"),
                                ("Esc", "Clear input draft"),
                            ] {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{key:<28}"))
                                            .color(Colors::TEXT_PRIMARY)
                                            .font(FontId::new(11.5, FontFamily::Monospace)),
                                    );
                                    ui.label(
                                        RichText::new(desc)
                                            .color(Colors::TEXT_MUTED)
                                            .font(FontId::new(11.5, FontFamily::Proportional)),
                                    );
                                });
                            }
                        });
                });

            if let Some(ex) = example_to_load {
                self.state.draft_input = ex;
                self.state.request_input_focus = true;
                self.help_open = false;
            }
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
            tracing::warn!("Failed to save session in App::save: {}", e);
        }
    }
}

impl Drop for CasetteApp {
    fn drop(&mut self) {
        if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
            tracing::warn!("Failed to save session in Drop: {}", e);
        }
    }
}
