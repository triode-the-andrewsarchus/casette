//! Casette desktop application main entry point.

use casette_gui::app::CasetteApp;
use casette_gui::model::{AsyncMessage, GuiAction, WorkerStatus};
use casette_kernel::SageKernel;
use eframe::NativeOptions;
use std::sync::mpsc;
use std::time::Instant;
use tracing::{error, info};

fn extract_reusable_expression(compiled_sage: &str) -> Option<String> {
    let sage = compiled_sage.trim();
    if sage.is_empty()
        || sage.starts_with("plot(")
        || sage.starts_with("implicit_plot(")
        || sage.starts_with("parametric_plot(")
    {
        return None;
    }

    // Echoed assignment check: "A = ...\nA"
    let lines: Vec<&str> = sage
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() == 2 {
        if let Some((lhs, _)) = lines[0].split_once('=') {
            let target = lhs.trim();
            if lines[1] == target && friendly_compiler::variables::is_plausible_variable(target) {
                return Some(target.to_string());
            }
        }
    }

    // Multiline non-assignment cannot be safely reused as a single expression
    if lines.len() > 1 {
        return None;
    }

    Some(sage.to_string())
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting Casette Desktop Application");

    // Channels for UI <-> Tokio async worker communication
    let (action_tx, action_rx) = mpsc::channel::<GuiAction>();
    let (msg_tx, msg_rx) = mpsc::channel::<AsyncMessage>();

    // Start background Tokio multi-thread runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let msg_tx_clone = msg_tx.clone();
    rt.spawn(async move {
        info!("launching SageKernel background manager");
        let kernel = match SageKernel::launch_discovered().await {
            Ok(k) => {
                let pid = k.worker_pid().unwrap_or(0);
                let _ = msg_tx_clone.send(AsyncMessage::WorkerReady {
                    pid,
                    version: "SageMath (Linux)".into(),
                });
                k
            }
            Err(e) => {
                error!("failed to launch Sage kernel: {}", e);
                let _ = msg_tx_clone.send(AsyncMessage::ErrorOccurred(e.to_string()));
                return;
            }
        };

        while let Ok(action) = action_rx.recv() {
            match action {
                GuiAction::Evaluate {
                    row_number,
                    input: _,
                    compiled_code,
                    numeric,
                } => {
                    let row_num = row_number;
                    let start_time = Instant::now();
                    match kernel.eval(&compiled_code, numeric, None).await {
                        Ok(response) => {
                            let duration = start_time.elapsed();
                            let is_ok = response.ok && response.kind.as_deref() != Some("error");

                            let _ = msg_tx_clone.send(AsyncMessage::EvalFinished {
                                row_number: row_num,
                                response: Box::new(response),
                                duration,
                            });

                            // If execution succeeded, register reference in Sage worker dictionary
                            if is_ok {
                                if let Some(expr) = extract_reusable_expression(&compiled_code) {
                                    let assign_code = format!(
                                        "__casette_tape_refs[{}] = ({})\n",
                                        row_num, expr
                                    );
                                    let _ = kernel.eval(&assign_code, false, None).await;
                                }
                            }
                        }
                        Err(e) => {
                            error!("eval failed: {}", e);
                            let _ = msg_tx_clone.send(AsyncMessage::ErrorOccurred(e.to_string()));
                        }
                    }
                }
                GuiAction::SyncTapeRefs(refs) => {
                    let mut code = String::from("__casette_tape_refs = {}\n");
                    for (num, expr) in refs {
                        code.push_str(&format!("__casette_tape_refs[{}] = ({})\n", num, expr));
                    }
                    let _ = kernel.eval(&code, false, None).await;
                }
                GuiAction::SyncDefinitions(defs) => {
                    if !defs.is_empty() {
                        let combined_code: String = defs
                            .iter()
                            .map(|d| d.statement.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let _ = kernel.eval(&combined_code, false, None).await;
                    }
                }
                GuiAction::DeleteSymbol(name) => {
                    let cmd = format!("_casette_delete_symbol('{}')\n", name);
                    let _ = kernel.eval(&cmd, false, None).await;
                    if let Ok(syms) = kernel.symbols().await {
                        let _ = msg_tx_clone.send(AsyncMessage::SymbolsUpdated(syms));
                    }
                }
                GuiAction::ClearTape => {
                    let _ = kernel.eval("__casette_tape_refs = {}\n", false, None).await;
                }
                GuiAction::Ping => {
                    let _ = kernel.ping().await;
                }
                GuiAction::FetchSymbols => {
                    if let Ok(syms) = kernel.symbols().await {
                        let _ = msg_tx_clone.send(AsyncMessage::SymbolsUpdated(syms));
                    }
                }
                GuiAction::SetPrecision(digits) => {
                    if let Ok(p) = kernel.config_precision(digits).await {
                        let _ = msg_tx_clone.send(AsyncMessage::PrecisionUpdated(p));
                    }
                }
                GuiAction::Interrupt => {
                    let _ = kernel.interrupt();
                }
                GuiAction::RestartWorker => {
                    let _ =
                        msg_tx_clone.send(AsyncMessage::StatusChanged(WorkerStatus::Restarting));
                    if let Ok(()) = kernel.restart().await {
                        let pid = kernel.worker_pid().unwrap_or(0);
                        let _ = msg_tx_clone.send(AsyncMessage::WorkerReady {
                            pid,
                            version: "SageMath (Linux)".into(),
                        });
                    }
                }
            }
        }
    });

    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("Casette"),
        ..Default::default()
    };

    eframe::run_native(
        "Casette",
        native_options,
        Box::new(|cc| Ok(Box::new(CasetteApp::new(cc, action_tx, msg_rx)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}
