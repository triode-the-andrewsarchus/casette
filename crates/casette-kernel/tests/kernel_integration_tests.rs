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

    // 3. Test symbols query (including shadowed Sage builtin like n)
    kernel
        .eval("n = 104729", false, None)
        .await
        .expect("eval n assignment");
    let symbols = kernel.symbols().await.expect("symbols");
    assert!(symbols.iter().any(|s| s.name == "x"));
    let n_sym = symbols
        .iter()
        .find(|s| s.name == "n")
        .expect("n should be reported in symbols after reassignment");
    assert_eq!(n_sym.kind, "integer");

    // 4. Test precision config
    let prec = kernel.config_precision(20).await.expect("config precision");
    assert_eq!(prec, 20);

    // 4b. Test Problem 020 classification traps:
    // (a) Gaussian integer (3 + 4*I) is classified as "complex", not unknown
    let gauss_resp = kernel
        .eval("3 + 4*I", false, None)
        .await
        .expect("eval 3 + 4*I");
    assert_eq!(gauss_resp.kind.as_deref(), Some("complex"));

    // (b) solve(...) returns Sequence_generic which subclasses list -> classified as "list"
    let solve_resp = kernel
        .eval("solve(x^2 == 1, x)", false, None)
        .await
        .expect("eval solve");
    assert_eq!(solve_resp.kind.as_deref(), Some("list"));

    // (c) bool is int subclass -> True must classify as "boolean"
    let bool_resp = kernel.eval("True", false, None).await.expect("eval True");
    assert_eq!(bool_resp.kind.as_deref(), Some("boolean"));

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

    // 6. Test units: solve with units and conversion
    let solve_units = kernel
        .eval("solve(32*mi*x^2 == 52*mi^2, x)", false, None)
        .await
        .expect("eval solve units");
    assert!(solve_units.ok);
    let solve_plain = solve_units.plain.as_deref().unwrap_or("");
    assert!(solve_plain.contains("sqrt(mile)"), "expected solve output to contain sqrt(mile), got: {}", solve_plain);

    let ratio_resp = kernel
        .eval("__casette_convert(50*km / (32*mi))", false, None)
        .await
        .expect("eval ratio units");
    assert!(ratio_resp.ok);
    assert_eq!(ratio_resp.plain.as_deref(), Some("390625/402336"));

    // 7. Test shutdown
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

#[tokio::test]
async fn test_kernel_restart_chains_preludes() {
    let env = SageDiscovery::discover().await.expect("discover sage");
    let config = KernelConfig {
        python_path: env.python_path,
        worker_script: env.worker_script,
        startup_timeout: Duration::from_secs(15),
        default_eval_timeout: Duration::from_secs(15),
        initial_precision: Some(10),
        ..Default::default()
    };

    let kernel = SageKernel::launch(config).await.expect("launch kernel");

    // 1. Initial eval with predefined variable x
    let r1 = kernel.eval("expand((x + 1)^2)", false, None).await.expect("eval r1");
    assert!(r1.ok);
    assert_eq!(r1.plain.as_deref(), Some("x^2 + 2*x + 1"));

    // 2. Restart kernel
    kernel.restart().await.expect("restart");

    // 3. Problem 002 verification: immediate submission after restart must not race boot preludes
    let r2 = kernel.eval("expand((x + 1)^2)", false, None).await.expect("eval r2");
    assert!(r2.ok, "expand((x+1)^2) must succeed immediately after restart without NameError");
    assert_eq!(r2.plain.as_deref(), Some("x^2 + 2*x + 1"));

    kernel.shutdown().await.expect("shutdown");
}

