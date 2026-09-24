//! Casette desktop application main entry point.

use casette_gui::app::CasetteApp;
use casette_gui::model::{AsyncMessage, GuiAction, WorkerStatus};
use casette_kernel::SageKernel;
use eframe::NativeOptions;
use std::sync::mpsc;
use std::time::Instant;
use tracing::{error, info};

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

        let mut row_counter = 1usize;

        while let Ok(action) = action_rx.recv() {
            match action {
                GuiAction::Evaluate { input, numeric } => {
                    let row_num = row_counter;
                    row_counter += 1;

                    // Compile friendly input to Sage
                    let compiled_code = match friendly_compiler::compile(&input) {
                        friendly_compiler::CompileResult::Success { generated_sage, .. } => {
                            generated_sage
                        }
                        friendly_compiler::CompileResult::Bypass { raw_sage } => raw_sage,
                        _ => input.clone(),
                    };

                    let start_time = Instant::now();
                    match kernel.eval(&compiled_code, numeric, None).await {
                        Ok(response) => {
                            let duration = start_time.elapsed();
                            let _ = msg_tx_clone.send(AsyncMessage::EvalFinished {
                                row_number: row_num,
                                response: Box::new(response),
                                duration,
                            });
                        }
                        Err(e) => {
                            error!("eval failed: {}", e);
                            let _ = msg_tx_clone.send(AsyncMessage::ErrorOccurred(e.to_string()));
                        }
                    }
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
