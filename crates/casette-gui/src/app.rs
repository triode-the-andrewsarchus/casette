//! Main eframe application coordinating GUI state and background kernel workers.

use crate::model::{AsyncMessage, GuiAction, SessionState, WorkerStatus};
use crate::render::TextureCache;
use crate::theme::{apply_theme, Colors};
use crate::views::{render_header, render_input_pane, render_sidebar, render_tape, SidebarTab};
use eframe::App;
use egui::{
    Button, CentralPanel, Context, FontFamily, FontId, RichText, SidePanel, Stroke,
    TopBottomPanel, Window,
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
                tracing::warn!("Failed to load previous session from {:?}: {}", session_path, e);
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
                    if let Some(row) = self
                        .state
                        .rows
                        .iter_mut()
                        .find(|r| r.row_number == row_number)
                    {
                        row.complete(*response, duration);
                    }
                    self.state.worker_status = WorkerStatus::Ready;
                    let _ = self.action_tx.send(GuiAction::FetchSymbols);
                    if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
                        tracing::warn!("Failed to auto-save session: {}", e);
                    }
                }
                AsyncMessage::SymbolsUpdated(symbols) => {
                    self.state.symbols = symbols;
                }
                AsyncMessage::PrecisionUpdated(digits) => {
                    self.state.precision_digits = digits;
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
        let (compiled, compile_err) = if input.contains('\n') {
            (input.clone(), None)
        } else {
            match friendly_compiler::compile_with_refs(&input, &refs) {
                friendly_compiler::CompileResult::Success { generated_sage, .. } => {
                    (generated_sage, None)
                }
                friendly_compiler::CompileResult::Bypass { raw_sage } => (raw_sage, None),
                friendly_compiler::CompileResult::Error(err) => (input.clone(), Some(err)),
                friendly_compiler::CompileResult::Ambiguous { candidates } => {
                    let msg = format!("Ambiguous expression. Did you mean:\n{}", candidates.join("\n"));
                    (
                        input.clone(),
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
            if let Some(row) = self
                .state
                .rows
                .iter_mut()
                .find(|r| r.row_number == row_num)
            {
                row.complete(err_resp, std::time::Duration::from_millis(0));
            }
            self.state.worker_status = WorkerStatus::Ready;
            if let Err(e) = crate::persistence::save_session(&self.state, &self.session_path) {
                tracing::warn!("Failed to auto-save session: {}", e);
            }
            return;
        }

        self.state.worker_status = WorkerStatus::Busy;

        let _ = self.action_tx.send(GuiAction::Evaluate {
            row_number: row_num,
            input,
            compiled_code: compiled,
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

            render_header(
                ui,
                &mut self.state,
                &mut self.sidebar_visible,
                &mut self.doctor_open,
                &mut |digits| prec_change = Some(digits),
                &mut || clear_tape = true,
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

                    render_sidebar(
                        ui,
                        &mut self.state,
                        &mut self.current_sidebar_tab,
                        &mut self.symbol_filter,
                        &mut self.show_builtin_symbols,
                        &mut || refresh_syms = true,
                        &mut || restart_worker = true,
                        &mut |s| draft_to_load = Some(s),
                    );

                    if refresh_syms {
                        let _ = self.action_tx.send(GuiAction::FetchSymbols);
                    }
                    if restart_worker {
                        let _ = self.action_tx.send(GuiAction::RestartWorker);
                    }
                    if let Some(text) = draft_to_load {
                        self.state.draft_input = text;
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
