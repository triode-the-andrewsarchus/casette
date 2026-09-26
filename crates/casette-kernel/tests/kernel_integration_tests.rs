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

    // 2b. Test implicit multiplication: solve(x^2 + 5x + 6 == 0, x)
    let solve_resp = kernel
        .eval("solve(x^2 + 5x + 6 == 0, x)", false, None)
        .await
        .expect("eval solve with 5x");
    assert!(solve_resp.ok);
    let solve_plain = solve_resp.plain.as_deref().unwrap();
    assert!(solve_plain.contains("x == -3") && solve_plain.contains("x == -2"));

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

    // 4c. Test Problem 021 numeric approximation: native precision clamping and kind gating
    // (a) CC(3, 4) with 53 bits native precision does not raise on approx
    let cc_resp = kernel
        .eval("CC(3, 4)", false, None)
        .await
        .expect("eval CC(3, 4)");
    assert!(cc_resp.ok);
    assert!(cc_resp.approx.is_some());
    assert!(cc_resp.approx.as_deref().unwrap().contains("3.00"));

    // (b) Symbolic with free variables has NO scalar approx
    let free_sym_resp = kernel
        .eval("x^2 + 5*x + 6", false, None)
        .await
        .expect("eval free sym");
    assert!(free_sym_resp.ok);
    assert_eq!(free_sym_resp.approx, None);

    // (c) Matrix has NO scalar approx
    let mat_resp = kernel
        .eval("matrix([[1, 2], [3, 4]])", false, None)
        .await
        .expect("eval matrix");
    assert!(mat_resp.ok);
    assert_eq!(mat_resp.approx, None);

    // (d) Force-numeric mode on sqrt(2) makes decimal primary, exact_value preserved
    let num_resp = kernel
        .eval("sqrt(2)", true, Some(10))
        .await
        .expect("eval sqrt(2) force-numeric");
    assert!(num_resp.ok);
    assert_eq!(num_resp.primary_is_approx, Some(true));
    assert_eq!(num_resp.exact_value.as_deref(), Some("sqrt(2)"));
    assert!(num_resp.plain.as_deref().unwrap().starts_with("1.414213562"));

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

    // 6a. Test solve with unit preservation and numeric approximation
    let solve_ft_exact = kernel
        .eval("solve(3*mi + x*ft == 7*km, x)", false, None)
        .await
        .expect("eval solve with foot unit");
    assert!(solve_ft_exact.ok);
    let exact_plain = solve_ft_exact.plain.as_deref().unwrap_or("");
    assert!(
        exact_plain.contains("2714960/381*foot"),
        "expected exact plain to have foot unit and fraction, got: {}",
        exact_plain
    );
    assert!(
        solve_ft_exact.approx.as_deref().unwrap_or("").contains("7125.87"),
        "expected exact response to provide numeric approx with foot unit, got: {:?}",
        solve_ft_exact.approx
    );
    assert_eq!(solve_ft_exact.primary_is_approx, Some(false));

    // Force-numeric mode on solve with units
    let solve_ft_num = kernel
        .eval("solve(3*mi + x*ft == 7*km, x)", true, Some(10))
        .await
        .expect("eval solve with foot unit numeric");
    assert!(solve_ft_num.ok);
    let num_plain = solve_ft_num.plain.as_deref().unwrap_or("");
    assert!(
        num_plain.contains("7125.87") && num_plain.contains("foot"),
        "expected numeric plain to contain approximation with foot unit, got: {}",
        num_plain
    );
    assert_eq!(solve_ft_num.primary_is_approx, Some(true));
    assert!(
        solve_ft_num.exact_value.as_deref().unwrap_or("").contains("2714960/381*foot"),
        "expected exact_value to preserve original exact fraction with unit, got: {:?}",
        solve_ft_num.exact_value
    );

    // 6b. Test Problem 025: private dup'd fd for protocol stream and OS-level fd 1/2 redirection
    let raw_fd_resp = kernel
        .eval(
            "import os; os.write(1, b\"RAW_FD1_DATA\\n\"); print(\"PYTHON_PRINT_DATA\"); 42",
            false,
            None,
        )
        .await
        .expect("eval raw fd and print write");
    assert!(raw_fd_resp.ok);
    assert_eq!(raw_fd_resp.plain.as_deref(), Some("42"));
    let captured_stdout = raw_fd_resp.stdout.as_deref().unwrap_or("");
    assert!(
        captured_stdout.contains("RAW_FD1_DATA"),
        "expected OS-level fd 1 write to be captured into stdout, got: {}",
        captured_stdout
    );
    assert!(
        captured_stdout.contains("PYTHON_PRINT_DATA"),
        "expected Python print to be captured into stdout, got: {}",
        captured_stdout
    );

    // 6c. Test Problem 026: value echo suppresses None and factor(x^4 - 1) unordered terms
    let print_resp = kernel
        .eval("print(\"suppress_none_test\")", false, None)
        .await
        .expect("eval bare print");
    assert!(print_resp.ok);
    assert_eq!(print_resp.kind.as_deref(), Some("none"));
    assert_eq!(print_resp.value, Some(false));
    assert_eq!(print_resp.plain.as_deref(), Some(""));
    assert!(print_resp
        .stdout
        .as_deref()
        .unwrap_or("")
        .contains("suppress_none_test"));

    let factor_resp = kernel
        .eval("factor(x^4 - 1)", false, None)
        .await
        .expect("eval factor x^4 - 1");
    assert!(factor_resp.ok);
    let factor_plain = factor_resp.plain.as_deref().unwrap_or("");
    assert!(
        factor_plain.contains("x - 1")
            && factor_plain.contains("x + 1")
            && factor_plain.contains("x^2 + 1")
    );

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

#[tokio::test]
async fn test_kernel_interactive_plot_data() {
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

    // Evaluate a plot command in Sage
    let resp = kernel
        .eval("plot(sin(x), (x, -pi, pi))", false, None)
        .await
        .expect("eval plot");
    assert!(resp.ok);
    assert_eq!(resp.kind.as_deref(), Some("plot"));

    // Verify structured plot_data payload
    let pd = resp.plot_data.expect("plot_data payload must be present");
    assert!(!pd.series.is_empty(), "must have at least one series");
    let s0 = &pd.series[0];
    assert!(s0.points.len() > 50, "series should have sampled points");
    assert!(pd.x_range[0] <= -3.0 && pd.x_range[1] >= 3.0);
    assert!(pd.y_range[0] <= -0.9 && pd.y_range[1] >= 0.9);

    // Verify local extrema were detected on the sine curve
    assert!(!s0.extrema.is_empty(), "sine wave must have detected extrema");
    let has_max = s0
        .extrema
        .iter()
        .any(|e| matches!(e.kind, casette_kernel::ExtremumKind::Maximum));
    let has_min = s0
        .extrema
        .iter()
        .any(|e| matches!(e.kind, casette_kernel::ExtremumKind::Minimum));
    assert!(has_max, "must detect local maximum");
    assert!(has_min, "must detect local minimum");

    // Also verify static artifacts are still produced for persistence/export
    assert!(resp.artifacts.iter().any(|a| a.r#type == "image"));

    kernel.shutdown().await.expect("shutdown");
}

