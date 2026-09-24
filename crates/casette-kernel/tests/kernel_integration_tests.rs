//! Integration tests for `casette-kernel` communicating with `worker.py`.

use casette_kernel::{KernelConfig, SageDiscovery, SageKernel};
use std::time::Duration;

#[tokio::test]
async fn test_kernel_boot_eval_ping() {
    let env = SageDiscovery::discover().await.expect("discover sage");
    let config = KernelConfig {
        python_path: env.python_path,
        worker_script: env.worker_script,
        startup_timeout: Duration::from_secs(15),
        default_eval_timeout: Duration::from_secs(30),
        initial_precision: Some(10),
        ..Default::default()
    };

    let kernel = SageKernel::launch(config).await.expect("launch kernel");
    assert!(kernel.is_alive());
    assert!(kernel.worker_pid().is_some());

    // 1. Test ping
    let latency = kernel.ping().await.expect("ping");
    assert!(latency < Duration::from_secs(5));

    // 2. Test eval: factor(x^2 - 1)
    let resp = kernel
        .eval("factor(x^2 - 1)", false, None)
        .await
        .expect("eval");
    assert!(resp.ok);
    assert_eq!(resp.kind.as_deref(), Some("symbolic"));
    let plain = resp.plain.as_deref().unwrap();
    assert!(plain.contains("x - 1") && plain.contains("x + 1"));

    // 3. Test symbols query
    let symbols = kernel.symbols().await.expect("symbols");
    // x should be recognized
    assert!(symbols.iter().any(|s| s.name == "x"));

    // 4. Test precision config
    let prec = kernel.config_precision(20).await.expect("config precision");
    assert_eq!(prec, 20);

    // 5. Test math artifact generation on integration
    let math_resp = kernel
        .eval("integrate(1/(1-x^2), x)", false, None)
        .await
        .expect("eval math");
    assert!(math_resp.ok);
    assert!(math_resp.latex.is_some());
    let math_art = math_resp
        .artifacts
        .iter()
        .find(|a| a.r#type == "math")
        .expect("should produce a math artifact");
    assert_eq!(math_art.format, "svg");
    let art_path = math_art.path.as_deref().expect("artifact path");
    assert!(std::path::Path::new(art_path).exists(), "SVG artifact file should exist");

    // 6. Test shutdown
    kernel.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn test_kernel_interrupt() {
    let env = SageDiscovery::discover().await.expect("discover sage");
    let config = KernelConfig {
        python_path: env.python_path,
        worker_script: env.worker_script,
        startup_timeout: Duration::from_secs(15),
        default_eval_timeout: Duration::from_secs(10),
        initial_precision: Some(10),
        ..Default::default()
    };

    let kernel = SageKernel::launch(config).await.expect("launch kernel");

    // Launch a task that would take a long time
    let kernel_clone = kernel.clone();
    let eval_handle = tokio::spawn(async move {
        // Run a loop in Python that catches SIGINT
        kernel_clone
            .eval("import time; time.sleep(10)", false, None)
            .await
    });

    // Wait a brief moment to ensure worker entered eval
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Send interrupt
    kernel.interrupt().expect("interrupt");

    // The eval should return an interrupted envelope promptly
    let res = eval_handle.await.expect("join").expect("eval response");
    assert!(!res.ok);
    assert!(res.is_interrupted());

    // Kernel should still be alive and usable after interruption
    let ping_latency = kernel.ping().await.expect("ping after interrupt");
    assert!(ping_latency < Duration::from_secs(5));

    let eval_after = kernel
        .eval("1 + 1", false, None)
        .await
        .expect("eval after interrupt");
    assert!(eval_after.ok);
    assert_eq!(eval_after.plain.as_deref(), Some("2"));

    kernel.shutdown().await.expect("shutdown");
}
