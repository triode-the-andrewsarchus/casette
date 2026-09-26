//! Integration tests for in-process `WoxiKernel`.

use casette_kernel::WoxiKernel;
use std::time::Duration;

#[tokio::test]
async fn test_woxi_boot_eval_ping() {
    let kernel = WoxiKernel::launch().await.expect("launch woxi kernel");
    assert!(kernel.is_alive());
    assert!(kernel.worker_pid().is_some());

    // 1. Test ping
    let latency = kernel.ping().await.expect("ping");
    assert!(latency < Duration::from_secs(1));

    // 2. Test arithmetic & factor
    let resp = kernel
        .eval("factor(x^2 - 1)", false, None)
        .await
        .expect("eval");
    assert!(resp.ok);
    assert_eq!(resp.kind.as_deref(), Some("symbolic"));
    let plain = resp.plain.as_deref().unwrap();
    assert!(plain.contains("-1 + x") && plain.contains("1 + x") || plain.contains("x - 1"));

    // 3. Test calculus: differentiation & integration
    let diff_resp = kernel.eval("diff(x^3, x)", false, None).await.expect("diff");
    assert!(diff_resp.ok);
    assert_eq!(diff_resp.plain.as_deref(), Some("3*x^2"));

    let int_resp = kernel
        .eval("integrate(1/(1-x^2), x)", false, None)
        .await
        .expect("integrate");
    assert!(int_resp.ok);
    let int_plain = int_resp.plain.as_deref().unwrap();
    assert!(int_plain.contains("Log"));
    assert_eq!(
        int_resp.latex.as_deref(),
        Some(r"\frac{1}{2} \log (x+1)-\frac{1}{2} \log (1-x)")
    );

    // 4. Test matrices and matrix properties
    let mat_resp = kernel
        .eval("matrix([[1, 2], [3, 4]]).det()", false, None)
        .await
        .expect("det");
    assert!(mat_resp.ok);
    assert_eq!(mat_resp.plain.as_deref(), Some("-2"));

    // 5. Test Physical Units & UnitSimplify (the core migration test!)
    let ratio_resp = kernel
        .eval("__casette_convert(50*km / (32*mi))", false, None)
        .await
        .expect("ratio units");
    assert!(ratio_resp.ok);
    assert_eq!(ratio_resp.plain.as_deref(), Some("390625/402336"));

    let conv_resp = kernel
        .eval("__casette_convert((100*km) / (2*hr), mph)", false, None)
        .await
        .expect("conversion units");
    assert!(conv_resp.ok);
    assert!(conv_resp.plain.as_deref().unwrap().contains("390625/12573"));

    // 6. Test symbols query
    kernel
        .eval("testSymbol = 104729", false, None)
        .await
        .expect("eval symbol");
    let symbols = kernel.symbols().await.expect("symbols");
    assert!(symbols.iter().any(|s| s.name == "testSymbol"));

    // 7. Test precision config
    let prec = kernel.config_precision(25).await.expect("config precision");
    assert_eq!(prec, 25);

    // 8. Test numeric approximation
    let num_resp = kernel.eval("1/3", true, Some(15)).await.expect("numeric");
    assert!(num_resp.ok);
    assert!(num_resp.plain.as_deref().unwrap().contains("0.333333333333333"));

    // 9. Test restart
    kernel.restart().await.expect("restart");
    let after_restart = kernel.eval("2 + 2", false, None).await.expect("after restart");
    assert!(after_restart.ok);
    assert_eq!(after_restart.plain.as_deref(), Some("4"));

    // 10. Shutdown
    kernel.shutdown().await.expect("shutdown");
    assert!(!kernel.is_alive());
}

#[tokio::test]
async fn test_woxi_solve_equation_with_units() {
    let kernel = WoxiKernel::launch().await.expect("launch woxi kernel");
    let resp = kernel
        .eval("solve((3*mi) + (x*ft) == (5*km), x)", false, None)
        .await
        .expect("eval");
    assert!(resp.ok);
    assert_eq!(resp.plain.as_deref(), Some("{{x -> 214960/381 ft}}"));
    let latex = resp.latex.as_deref().unwrap();
    assert!(latex.contains("214960") && latex.contains("381") && latex.contains("ft"));

    // Approx field is populated with converted numeric feet
    assert!(resp.approx.as_deref().unwrap().contains("564.199475") && resp.approx.as_deref().unwrap().contains("ft"));

    // Time equation solve in seconds
    let time_resp = kernel
        .eval("solve((3*hr) + (x*sec) == (12*days), x)", false, None)
        .await
        .expect("eval time");
    assert!(time_resp.ok);
    assert_eq!(time_resp.plain.as_deref(), Some("{{x -> 1026000 s}}"));

    // Feet, inches, yard equation: 2 ft + x in = 1 yard -> x = 12 in
    let yd_resp = kernel
        .eval("solve((2*ft) + (x*in) == (1*yard), x)", false, None)
        .await
        .expect("eval yard");
    assert!(yd_resp.ok);
    assert_eq!(yd_resp.plain.as_deref(), Some("{{x -> 12 in}}"));
    let yd_latex = yd_resp.latex.as_deref().unwrap();
    assert!(yd_latex.contains("12") && yd_latex.contains("in"));
    assert!(!yd_latex.contains("begin{array}"));

    kernel.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn test_woxi_interactive_plot_data() {
    let kernel = WoxiKernel::launch().await.expect("launch woxi kernel");

    // Evaluate 2D sine plot
    let resp = kernel
        .eval("plot(sin(x), (x, -pi, pi))", false, None)
        .await
        .expect("plot eval");
    assert!(resp.ok);
    assert_eq!(resp.kind.as_deref(), Some("plot"));
    assert!(resp.plot_data.is_some(), "expected plot_data payload");

    let plot_data = resp.plot_data.unwrap();
    assert_eq!(plot_data.series.len(), 1);
    let series = &plot_data.series[0];
    assert!(series.points.len() >= 50, "expected dense sampled points, got {}", series.points.len());

    // Check that extrema (minimum near -pi/2, maximum near pi/2) were detected
    assert!(series.extrema.len() >= 2, "expected at least 2 extrema, got {:?}", series.extrema);
    let has_max = series.extrema.iter().any(|e| e.kind == casette_kernel::ExtremumKind::Maximum && (e.y - 1.0).abs() < 0.05);
    let has_min = series.extrema.iter().any(|e| e.kind == casette_kernel::ExtremumKind::Minimum && (e.y - -1.0).abs() < 0.05);
    assert!(has_max, "expected local maximum near y=1");
    assert!(has_min, "expected local minimum near y=-1");

    kernel.shutdown().await.expect("shutdown");
}
