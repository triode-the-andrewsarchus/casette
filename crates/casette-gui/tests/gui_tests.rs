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

    let row2 = state.push_row("diff(#1, x)".into(), "diff(__casette_tape_refs[1], x)".into());
    assert_eq!(row2, 2);
    assert_eq!(state.rows.len(), 2);
    assert_eq!(state.next_row_number, 3);

    // Complete row 1
    let resp1: WorkerResponse = serde_json::from_str(r#"{"ok": true, "plain": "2", "kind": "integer"}"#)
        .expect("deserialize");
    state.rows[0].complete(resp1, Duration::from_millis(15));
    assert!(!state.rows[0].is_evaluating);
    assert_eq!(state.rows[0].result.as_ref().unwrap().plain.as_deref(), Some("2"));

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
        restored_state.rows[0].result.as_ref().unwrap().plain.as_deref(),
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
