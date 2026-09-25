# Problems & Hard-Won Lessons

All 28 historical problem notes from the V0 proof and pre-port phases have been audited, addressed, and verified with dedicated test coverage in the Rust workspace (`casette-rs`).

The individual markdown problem notes have been retired and their constraints are enforced directly by the test suites and runtime implementations:

- **Problems 003, 004, 005, 006, 007, 008, 012, 017, 018, 019, 027, 028 (Obsolete Swift/SwiftUI/AppKit/SwiftPM):** Retired as non-applicable to the Rust/egui/Tokio architecture.
- **Problem 001 (Schema Versioning & Restore UI State):** Enforced in `casette-gui/src/persistence.rs` (`CURRENT_SCHEMA_VERSION`, refusing future versions, resetting in-flight evaluations to `interrupted`, clearing live state on restore).
- **Problem 002 (Restart Re-init Chaining):** Enforced in `casette-kernel/src/client.rs` (serialized Tokio actor loop guarantees boot preludes `var('u, v, w, x, y, z...')` execute before any subsequent eval).
- **Problem 009 (Friendly Compiler Multiline Preservation):** Enforced in `casette-gui` (`app.rs`, `main.rs`, `input_view.rs` bypassing single-line flattening on multiline input containing `\n`).
- **Problem 010 (POSIX Child Signal Dispositions):** Enforced in `casette-kernel/src/client.rs` (`unsafe { cmd.pre_exec(...) }` clearing sigmask and resetting `SIGINT` to `SIG_DFL`).
- **Problem 011 (Missing/Expired Plot Artifacts):** Enforced in `casette-gui/src/render/math_view.rs` (graceful expired plot placeholder card).
- **Problem 013 (Symbolic Ring Exactness):** Enforced in `v0/01-worker-protocol/worker.py` (`_is_exact`, `_symbolic_has_inexact_atom` distinguishing exact constants `sin(1)` from floating approximations).
- **Problem 014 (Numeric Precision Clamping):** Enforced in `v0/01-worker-protocol/worker.py` (`_napprox_string` clamping bits to native `value.prec()`).
- **Problem 015 (Friendly Compiler Space Boundary Bypass):** Enforced in `crates/friendly-compiler` (preserving whitespace boundaries on symbol bypass rules and classifying definite integrals as symbolic).
- **Problem 016 (Symbol Table Object Identity Baseline):** Enforced in `v0/01-worker-protocol/worker.py` (`_SYMBOL_BASELINE = dict(NS)` diffing by `is not` to capture shadowed Sage builtins like `n`/`N`/`i`).
- **Problem 020 (Sage Result Type Classification):** Enforced in `v0/01-worker-protocol/worker.py` (`_classify` correctly handling `NumberFieldElement_gaussian`, `Sequence_generic`, and `bool` preceding `int`).
- **Problem 021 (Numeric Approximation Native Precision):** Enforced in `v0/01-worker-protocol/worker.py` (clamping precision to native bits and gating `approx` on scalar types).
- **Problem 022 (Interrupting Sage with cysignals):** Enforced in `worker.py` and `casette-kernel/src/client.rs` (preserving cysignals SIGINT handler for prompt C/GMP loop aborts and targeting worker PID).
- **Problem 023 (Worker Restart Generation Isolation):** Enforced in `casette-kernel/src/client.rs` (each generation has its own `WorkerProcess` and pipe streams without shared EOF flags).
- **Problem 024 (Process Group Kill for Bash Wrappers):** Enforced in `casette-kernel/src/client.rs` (`cmd.process_group(0)` and killing `-pid` in `kill_and_reap` to clean up wrapper scripts and worker processes).
- **Problem 025 (Private dup'd Protocol FDs & OS-level Redirection):** Enforced in `worker.py` (`_PROTO_OUT_FD = os.dup(1)` and `os.dup2` stream capture during eval).
- **Problem 026 (Preparsed AST Evaluation & None Suppression):** Enforced in `worker.py` (`_eval_code` compiling preparsed AST and suppressing `None` values).
