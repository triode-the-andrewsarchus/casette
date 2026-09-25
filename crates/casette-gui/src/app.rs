//! Main eframe application coordinating GUI state and background kernel workers.

use crate::model::{AsyncMessage, GuiAction, SessionState, WorkerStatus};
use crate::render::TextureCache;
use crate::theme::apply_theme;
use crate::views::{render_header, render_input_pane, render_sidebar, render_tape, SidebarTab};
use eframe::App;
use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};
use std::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

/// Main Casette GUI application structure.
pub struct CasetteApp {
    state: SessionState,
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

        Self {
            state: SessionState::default(),
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
        let compiled = match friendly_compiler::compile_with_refs(&input, &refs) {
            friendly_compiler::CompileResult::Success { generated_sage, .. } => generated_sage,
            friendly_compiler::CompileResult::Bypass { raw_sage } => raw_sage,
            _ => input.clone(),
        };

        let _row_num = self.state.push_row(input.clone(), compiled);
        self.state.worker_status = WorkerStatus::Busy;

        let _ = self.action_tx.send(GuiAction::Evaluate { input, numeric });
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
    }
}
