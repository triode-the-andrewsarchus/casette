//! Tests for Casette GUI model, session tape, and persistence.

use casette_gui::model::SessionState;
use casette_gui::persistence::{export_to_markdown, load_session, save_session};
use casette_kernel::WorkerResponse;
use std::time::Duration;

#[test]
fn test_session_state_push_and_clear() {
    let mut state = SessionState::default();
    assert_eq!(state.next_row_number, 1);
    assert!(state.rows.is_empty());

    let row1 = state.push_row("1 + 1".into(), "1 + 1".into());
    assert_eq!(row1, 1);
    assert_eq!(state.rows.len(), 1);
    assert_eq!(state.next_row_number, 2);

    let row2 = state.push_row(
        "diff(#1, x)".into(),
        "diff(__casette_tape_refs[1], x)".into(),
    );
    assert_eq!(row2, 2);
    assert_eq!(state.rows.len(), 2);
    assert_eq!(state.next_row_number, 3);

    // Complete row 1
    let resp1: WorkerResponse =
        serde_json::from_str(r#"{"ok": true, "plain": "2", "kind": "integer"}"#)
            .expect("deserialize");
    state.rows[0].complete(resp1, Duration::from_millis(15));
    assert!(!state.rows[0].is_evaluating);
    assert_eq!(
        state.rows[0].result.as_ref().unwrap().plain.as_deref(),
        Some("2")
    );

    // Clear tape
    state.clear_tape();
    assert_eq!(state.next_row_number, 1);
    assert!(state.rows.is_empty());
}

#[test]
fn test_session_persistence_roundtrip() {
    let mut state = SessionState::default();
    let row1 = state.push_row("factor(x^2 - 1)".into(), "factor(x^2 - 1)".into());
    let resp: WorkerResponse = serde_json::from_str(
        r#"{
            "ok": true,
            "kind": "symbolic",
            "plain": "(x - 1)*(x + 1)",
            "latex": "(x - 1)(x + 1)"
        }"#,
    )
    .expect("deserialize");
    state.rows[row1 - 1].complete(resp, Duration::from_millis(42));

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("casette_test_session_{}.json", std::process::id()));

    save_session(&state, &file_path).expect("save session");

    let mut restored_state = SessionState::default();
    load_session(&mut restored_state, &file_path).expect("load session");

    assert_eq!(restored_state.rows.len(), 1);
    assert_eq!(restored_state.rows[0].row_number, 1);
    assert_eq!(restored_state.rows[0].input_raw, "factor(x^2 - 1)");
    assert_eq!(
        restored_state.rows[0]
            .result
            .as_ref()
            .unwrap()
            .plain
            .as_deref(),
        Some("(x - 1)*(x + 1)")
    );
    assert!(!restored_state.rows[0].is_live); // Restored rows start as not live

    // Test markdown export
    let md = export_to_markdown(&restored_state);
    assert!(md.contains("# Casette Session Notebook"));
    assert!(md.contains("Row #1"));
    assert!(md.contains("(x - 1)*(x + 1)"));

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_persistence_refuses_newer_schema_and_recovers_running_rows() {
    let temp_dir = std::env::temp_dir();
    let future_path = temp_dir.join(format!("casette_future_session_{}.json", std::process::id()));

    // 1. Future schema refusal
    let future_json = r#"{
        "schema_version": 999,
        "rows": []
    }"#;
    std::fs::write(&future_path, future_json).expect("write future file");

    let mut state = SessionState::default();
    let res = load_session(&mut state, &future_path);
    assert!(res.is_err(), "should refuse newer schema version");
    assert!(
        res.unwrap_err().to_string().contains("refused session schema version 999"),
        "error message should cite refused schema version"
    );
    let _ = std::fs::remove_file(future_path);

    // 2. Restoring a row that was saved while evaluating (running)
    let running_path = temp_dir.join(format!("casette_running_session_{}.json", std::process::id()));
    let running_json = r#"{
        "schema_version": 1,
        "rows": [
            {
                "row_number": 1,
                "input_raw": "while True: pass",
                "compiled_sage": "while True: pass",
                "result": null,
                "is_evaluating": true,
                "duration": null,
                "is_live": true,
                "timestamp_secs": 1000
            }
        ]
    }"#;
    std::fs::write(&running_path, running_json).expect("write running file");

    let mut state2 = SessionState::default();
    load_session(&mut state2, &running_path).expect("load running session");
    assert_eq!(state2.rows.len(), 1);
    assert!(!state2.rows[0].is_evaluating, "restored row must not be evaluating/spinning");
    assert!(!state2.rows[0].is_live, "restored row must not be marked live");
    let result = state2.rows[0].result.as_ref().expect("result must be populated");
    assert_eq!(result.kind.as_deref(), Some("interrupted"));
    assert!(result.plain.as_deref().unwrap().contains("quit before this evaluation finished"));

    let _ = std::fs::remove_file(running_path);
}


#[test]
fn test_tape_references_expansion_and_reusable_expression() {
    let mut state = SessionState::default();

    // 1. Row 1: integral 1/(1-x^2)
    let r1 = state.push_row(
        "integral 1/(1-x^2)".into(),
        "integrate(1/(1-x^2), x)".into(),
    );
    let resp1: WorkerResponse = serde_json::from_str(
        r#"{
            "ok": true,
            "kind": "symbolic",
            "plain": "1/2*log(x + 1) - 1/2*log(x - 1)"
        }"#,
    )
    .expect("deserialize");
    state.rows[r1 - 1].complete(resp1, Duration::from_millis(30));

    // Reusable expression should be the integration expression
    assert_eq!(
        state.rows[0].reusable_expression().as_deref(),
        Some("integrate(1/(1-x^2), x)")
    );

    // 2. Row 2: failed division by zero (should be skipped in references)
    let r2 = state.push_row("1/0".into(), "1/0".into());
    let resp2: WorkerResponse =
        WorkerResponse::new_error("ZeroDivisionError".into(), "division by zero".into(), None);
    state.rows[r2 - 1].complete(resp2, Duration::from_millis(1));
    assert!(state.rows[1].reusable_expression().is_none());

    // 3. Row 3: echoed assignment "a = 5\na"
    let r3 = state.push_row("a = 5".into(), "a = 5\na".into());
    let resp3: WorkerResponse =
        serde_json::from_str(r#"{"ok": true, "plain": "5", "kind": "integer"}"#)
            .expect("deserialize");
    state.rows[r3 - 1].complete(resp3, Duration::from_millis(5));
    // Reusable expression should be the assigned variable name "a"
    assert_eq!(state.rows[2].reusable_expression().as_deref(), Some("a"));

    // Check tape reference table
    let refs = state.tape_references();
    assert!(refs.entries().contains_key(&1));
    assert!(!refs.entries().contains_key(&2)); // error row skipped
    assert!(refs.entries().contains_key(&3));

    // 4. Test expanding "plot #1, x=-10..10"
    let lowered = friendly_compiler::compile_with_refs("plot #1, x=-10..10", &refs);
    if let friendly_compiler::CompileResult::Success {
        generated_sage,
        required_variables,
    } = lowered
    {
        assert_eq!(generated_sage, "plot(__casette_tape_refs[1], (x, -10, 10))");
        assert_eq!(required_variables, vec!["x"]);
    } else {
        panic!("expected success for plot #1, got {:?}", lowered);
    }

    // 5. Test expanding #2 (should fail with missing reference)
    let bad_ref = friendly_compiler::compile_with_refs("plot #2, x=-10..10", &refs);
    assert!(matches!(
        bad_ref,
        friendly_compiler::CompileResult::Error(_)
    ));

    // 6. Test assignment code generated for row 1
    let assign_code = refs.assignment_code(1).expect("assignment code");
    assert!(assign_code.contains("__casette_tape_refs[1] = (integrate(1/(1-x^2), x))"));
    assert!(assign_code.contains("globals().get('__casette_tape_refs'"));
}

#[test]
fn test_math_rendering_and_texture_cache() {
    let ctx = egui::Context::default();
    let mut cache = casette_gui::render::TextureCache::new();

    // Test on-the-fly LaTeX rendering of integral result
    let latex = r"\frac{1}{2} \log\left(x + 1\right) - \frac{1}{2} \log\left(x - 1\right)";
    let handle = cache.get_or_render_latex(&ctx, latex);
    assert!(handle.is_some(), "LaTeX should be rendered to a texture");
    let texture = handle.unwrap();
    let size = texture.size_vec2();
    assert!(size.x > 50.0);
    assert!(size.y > 10.0);
}

#[test]
fn test_action_buttons_and_approx_compilation() {
    let mut state = SessionState::default();
    let r1 = state.push_row("355/113".into(), "355/113".into());
    let resp1: WorkerResponse = serde_json::from_str(
        r#"{
            "ok": true,
            "kind": "rational",
            "plain": "355/113",
            "latex": "\\frac{355}{113}"
        }"#,
    )
    .expect("deserialize");
    state.rows[r1 - 1].complete(resp1, Duration::from_millis(5));

    let refs = state.tape_references();

    // 1. Approx action snippet: "approx #1"
    let approx_compiled = friendly_compiler::compile_with_refs("approx #1", &refs);
    if let friendly_compiler::CompileResult::Success { generated_sage, .. } = approx_compiled {
        assert_eq!(generated_sage, "N(__casette_tape_refs[1])");
    } else {
        panic!("expected success for approx #1, got {:?}", approx_compiled);
    }

    // 2. Diff action snippet: "diff(#1, x)"
    let diff_compiled = friendly_compiler::compile_with_refs("diff(#1, x)", &refs);
    if let friendly_compiler::CompileResult::Bypass { raw_sage } = diff_compiled {
        assert_eq!(raw_sage, "diff(__casette_tape_refs[1], x)");
    } else {
        panic!("expected bypass for diff(#1, x), got {:?}", diff_compiled);
    }

    // 3. Integrate action snippet: "integral(#1, x)"
    let int_compiled = friendly_compiler::compile_with_refs("integral(#1, x)", &refs);
    if let friendly_compiler::CompileResult::Bypass { raw_sage } = int_compiled {
        assert_eq!(raw_sage, "integral(__casette_tape_refs[1], x)");
    } else {
        panic!("expected bypass for integral(#1, x), got {:?}", int_compiled);
    }

    // 4. Simplify action snippet: "simplify(#1)"
    let simp_compiled = friendly_compiler::compile_with_refs("simplify(#1)", &refs);
    if let friendly_compiler::CompileResult::Bypass { raw_sage } = simp_compiled {
        assert_eq!(raw_sage, "simplify(__casette_tape_refs[1])");
    } else {
        panic!("expected bypass for simplify(#1), got {:?}", simp_compiled);
    }
}

