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
    assert_eq!(restored_state.history, vec!["factor(x^2 - 1)"]);

    // Test markdown export
    let md = export_to_markdown(&restored_state);
    assert!(md.contains("# Casette Session Notebook"));
    assert!(md.contains("Row #1"));
    assert!(md.contains("(x - 1)*(x + 1)"));

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_default_session_path_resolution() {
    let p = casette_gui::persistence::default_session_path();
    assert!(p.to_string_lossy().contains("last-session.json") || p.to_string_lossy().contains("casette_last_session.json"));
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

#[test]
fn test_force_numeric_result_handling() {
    let resp: WorkerResponse = serde_json::from_str(
        r#"{
            "ok": true,
            "kind": "symbolic",
            "plain": "1.41421356237310",
            "latex": "1.41421356237310",
            "repr": "1.41421356237310",
            "exact": true,
            "primary_is_approx": true,
            "exact_value": "sqrt(2)"
        }"#,
    )
    .expect("deserialize");

    assert_eq!(resp.primary_is_approx, Some(true));
    assert_eq!(resp.exact_value.as_deref(), Some("sqrt(2)"));
    assert_eq!(resp.plain.as_deref(), Some("1.41421356237310"));
}

#[test]
fn test_multiline_raw_sage_preserves_newlines() {
    let mut state = SessionState::default();
    let multiline_input = "a = 5\nb = 10\na * b".to_string();

    // Problem 009: When submitting multiline input, bypass friendly single-line compiler
    // so newlines are strictly preserved rather than flattened to spaces.
    let compiled = if multiline_input.contains('\n') {
        multiline_input.clone()
    } else {
        let refs = state.tape_references();
        match friendly_compiler::compile_with_refs(&multiline_input, &refs) {
            friendly_compiler::CompileResult::Success { generated_sage, .. } => generated_sage,
            friendly_compiler::CompileResult::Bypass { raw_sage } => raw_sage,
            _ => multiline_input.clone(),
        }
    };

    let row_num = state.push_row(multiline_input.clone(), compiled);
    assert_eq!(row_num, 1);
    assert_eq!(state.rows[0].compiled_sage, "a = 5\nb = 10\na * b");
    assert!(state.rows[0].compiled_sage.contains('\n'));
}

#[test]
fn test_interactive_plot_model_and_reusable_expression() {
    use casette_kernel::{ExtremumKind, LocalExtremum, PlotDataPayload, PlotSeriesPayload};

    let mut state = SessionState::default();
    let row_num = state.push_row(
        "plot sin(x), x=-pi..pi".into(),
        "plot(sin(x), (x, -pi, pi))".into(),
    );

    let resp = WorkerResponse {
        ok: true,
        kind: Some("plot".into()),
        plot_data: Some(PlotDataPayload {
            title: None,
            x_label: None,
            y_label: None,
            series: vec![PlotSeriesPayload {
                label: Some("sin(x)".into()),
                points: vec![[-3.0, 0.0], [-1.5, -1.0], [0.0, 0.0], [1.5, 1.0], [3.0, 0.0]],
                color: [56, 139, 255],
                is_scatter: false,
                extrema: vec![
                    LocalExtremum {
                        x: -1.5,
                        y: -1.0,
                        kind: ExtremumKind::Minimum,
                    },
                    LocalExtremum {
                        x: 0.0,
                        y: 0.0,
                        kind: ExtremumKind::ZeroCrossing,
                    },
                    LocalExtremum {
                        x: 1.5,
                        y: 1.0,
                        kind: ExtremumKind::Maximum,
                    },
                ],
            }],
            x_range: [-3.0, 3.0],
            y_range: [-1.0, 1.0],
        }),
        ..Default::default()
    };

    state.rows[row_num - 1].complete(resp, Duration::from_millis(100));

    // A plot row must NOT provide a reusable expression for arithmetic tape references
    assert_eq!(state.rows[row_num - 1].reusable_expression(), None);
}

#[test]
fn test_restored_session_row_number_continuity() {
    let temp_dir = std::env::temp_dir();
    let session_path = temp_dir.join(format!("casette_continuity_test_{}.json", std::process::id()));

    let json = r#"{
        "schema_version": 1,
        "rows": [
            {
                "row_number": 1,
                "input_raw": "2 + 2",
                "compiled_sage": "2 + 2",
                "result": {
                    "ok": true,
                    "kind": "integer",
                    "plain": "4"
                },
                "is_evaluating": false,
                "duration": { "secs": 0, "nanos": 5000000 },
                "is_live": false,
                "timestamp_secs": 1000
            },
            {
                "row_number": 2,
                "input_raw": "3 * 3",
                "compiled_sage": "3 * 3",
                "result": {
                    "ok": true,
                    "kind": "integer",
                    "plain": "9"
                },
                "is_evaluating": false,
                "duration": { "secs": 0, "nanos": 6000000 },
                "is_live": false,
                "timestamp_secs": 1001
            }
        ]
    }"#;
    std::fs::write(&session_path, json).expect("write session");

    let mut state = SessionState::default();
    load_session(&mut state, &session_path).expect("load session");
    let _ = std::fs::remove_file(session_path);

    assert_eq!(state.rows.len(), 2);
    assert_eq!(state.next_row_number, 3, "next row must be 3");

    // Tape references from restored rows must be available
    let refs = state.tape_references();
    assert_eq!(refs.entries().len(), 2);
    assert_eq!(refs.entries().get(&1).map(String::as_str), Some("2 + 2"));
    assert_eq!(refs.entries().get(&2).map(String::as_str), Some("3 * 3"));

    // New evaluation pushes row 3
    let new_row_num = state.push_row("#1 + #2".into(), "__casette_tape_refs[1] + __casette_tape_refs[2]".into());
    assert_eq!(new_row_num, 3, "new row number must match state.next_row_number");
    assert!(state.rows[2].is_evaluating, "new row should be marked evaluating");

    // When worker returns EvalFinished for row 3, it correctly completes row 3
    let finish_resp = WorkerResponse {
        ok: true,
        kind: Some("integer".into()),
        plain: Some("13".into()),
        ..Default::default()
    };
    let row = state.rows.iter_mut().find(|r| r.row_number == new_row_num).expect("find row 3");
    row.complete(finish_resp, Duration::from_millis(15));

    assert!(!state.rows[2].is_evaluating, "row 3 must no longer be evaluating");
    assert_eq!(state.rows[2].result.as_ref().unwrap().plain.as_deref(), Some("13"));
}

#[test]
fn test_user_definition_extraction_and_state_management() {
    use casette_gui::model::extract_definitions;

    // 1. Variable assignment
    let defs = extract_definitions("a = 5", "a = 5\na");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].name, "a");
    assert_eq!(defs[0].statement, "a = 5");

    // 2. Function definition
    let f_defs = extract_definitions("f(x) = x^2", "f(x) = x^2\nf");
    assert_eq!(f_defs.len(), 1);
    assert_eq!(f_defs[0].name, "f");
    assert_eq!(f_defs[0].statement, "f(x) = x^2");

    // 3. Symbolic var declaration
    let var_defs = extract_definitions("var('t, s')", "var('t, s')");
    assert_eq!(var_defs.len(), 2);
    assert_eq!(var_defs[0].name, "t");
    assert_eq!(var_defs[0].statement, "var('t')");
    assert_eq!(var_defs[1].name, "s");
    assert_eq!(var_defs[1].statement, "var('s')");

    // 4. SessionState set and remove
    let mut state = SessionState::default();
    state.set_definition("a", "a = 5");
    state.set_definition("b", "b = 10");
    state.set_definition("a", "a = 42"); // overwrite preserves position or updates
    assert_eq!(state.user_definitions.len(), 2);
    assert_eq!(state.user_definitions[0].statement, "a = 42");

    state.remove_definition("a");
    assert_eq!(state.user_definitions.len(), 1);
    assert_eq!(state.user_definitions[0].name, "b");
}

#[test]
fn test_definition_persistence_across_resets_and_tape_clears() {
    let mut state = SessionState::default();
    state.set_definition("x_init", "x_init = 100");
    state.set_definition("k", "k = 5");

    let row1 = state.push_row("x_init = 100".into(), "x_init = 100\nx_init".into());
    let resp: WorkerResponse = serde_json::from_str(r#"{"ok": true, "plain": "100", "kind": "integer"}"#).unwrap();
    state.rows[row1 - 1].complete(resp, Duration::from_millis(5));

    // Clear tape must preserve definitions!
    state.clear_tape();
    assert_eq!(state.rows.len(), 0);
    assert_eq!(state.user_definitions.len(), 2);
    assert_eq!(state.user_definitions[0].name, "x_init");

    // Persistence roundtrip
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("casette_def_test_{}.json", std::process::id()));

    save_session(&state, &file_path).expect("save session");

    let mut restored = SessionState::default();
    load_session(&mut restored, &file_path).expect("load session");
    let _ = std::fs::remove_file(file_path);

    assert_eq!(restored.user_definitions.len(), 2);
    assert_eq!(restored.user_definitions[0].name, "x_init");
    assert_eq!(restored.user_definitions[0].statement, "x_init = 100");
    assert_eq!(restored.user_definitions[1].name, "k");
    assert_eq!(restored.user_definitions[1].statement, "k = 5");
}
