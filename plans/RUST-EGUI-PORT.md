# Rust & egui Port Plan for Casette (Linux-Targeted)

## Executive Summary

This plan defines the architectural migration of **Casette** from a macOS SwiftUI/AppKit desktop app to a high-performance, native Linux desktop application written in **Rust** using **egui** (`eframe`).

The port preserves the core essence of Casette:
- A persistent **Session Tape** with row referencing (`#1`, `#2`, etc.).
- A long-lived **SageMath background process** speaking JSONL over stdio.
- A **Friendly Compiler** translating user shorthand (e.g. `diff(#1, x)`, `factor EXPR`, matlab matrices `[1 2; 3 4]`, and stats distributions) into Sage expressions on the fly.
- Rich math typesetting rendered offline.
- Interactive sidebars for live session symbols, history, and row inspections.

---

## Architectural Decisions (from Requirements Alignment)

1. **Target Platform:**
   - **Linux-only** primary target (running natively with X11 / Wayland via `winit` / `eframe`). Any cross-platform support (macOS/Windows) is an incidental benefit, not a design constraint.

2. **Repository & Crate Structure:**
   - Cargo workspace at repository root:
     - `crates/friendly-compiler`: Pure library port of `FriendlyCompiler` with complete 1:1 test parity (porting all 69 unit tests from Swift).
     - `crates/casette-kernel`: Asynchronous process manager and JSONL client for the Sage worker (`worker.py`). Handles process groups, SIGINT interrupts, timeouts, ping/pong, and Sage path discovery on Linux (`/usr/bin/sage`, `python3 -c "import sage.all"`, etc.).
     - `crates/casette-gui`: Native desktop GUI built with `eframe` (egui), custom styling, session state, math rendering, and UI controls.

3. **Math & Formula Rendering:**
   - Decoupled `MathRenderer` trait.
   - Vector/SVG rasterization using offline rendering (e.g. KaTeX / Typst Math / Rex translated to SVG/pixmap and rendered with `resvg` / `egui_extras`).
   - Zero WebViews or WebKitGTK runtime dependencies.
   - Seamless caching of rendered math textures in egui memory.

4. **Process Concurrency Model:**
   - Background multi-threaded **Tokio runtime** driving child process I/O and process-group control.
   - Decoupled, lock-free communication between Tokio and the egui main thread via bounded mpsc channels.
   - Waking up the GUI thread via `egui::Context::request_repaint()`.

5. **Session Persistence:**
   - Clean-sheet format using `serde_json` tailored for Rust idioms, storing session tape entries, inputs, timestamp metadata, and cached outputs.

6. **Swift Retirement Strategy:**
   - Swift code is retained temporarily during Phase 1 solely to reference and guarantee 100% parity on the 69 compiler test cases.
   - Once `crates/friendly-compiler` passes all ported tests, all legacy Swift code (`Package.swift`, `Sources/`, `Tests/`, `v0/04*`, etc.) and macOS build scripts will be deleted.

---

## Phased Implementation Roadmap

### Phase 0: Workspace Scaffolding & Setup
- Initialize root `Cargo.toml` workspace.
- Setup member crates:
  - `crates/friendly-compiler`
  - `crates/casette-kernel`
  - `crates/casette-gui`
- Configure workspace-level lints, dependencies (`tokio`, `serde`, `serde_json`, `eframe`, `egui`, `resvg`, `thiserror`).

### Phase 1: `friendly-compiler` Crate & Test Parity
- Implement Lexer (`FriendlyLexer`):
  - Tokenization of identifiers, numbers, operators, brackets, strings, tape references (`#1`, `#14`).
  - Matlab-style matrix brackets `[1 2; 3 4]`.
- Implement Parser & AST/IR (`FriendlyParser`, `FriendlyIR`):
  - Commands: `factor`, `expand`, `simplify`, `solve`, `derivative` / `diff`, `integral` (indefinite, definite, double), `limit`, `taylor`, `plot`, `matrix`, `eigenvalues`, etc.
  - Probability / statistics forms: `normal(mean, sd)`, `binomial(n, p)`, range queries.
  - `#ROW` tape reference tracking and resolution.
- Implement Lowering (`FriendlyLowering`):
  - Lowering IR to single executable Sage statements.
  - Automatic `requiredVariables` discovery (e.g. free variables needing `var(...)` definitions).
  - Raw Sage bypass rule.
- Port entire 69-test suite from Swift (`Tests/FriendlyCompilerTests/`) to `cargo test` in Rust.
- **Gate:** All 69 tests pass.

### Phase 2: `casette-kernel` Crate & Sage Communication
- Port `worker.py` launch and communication protocol:
  - Verify execution on Linux (`python3 worker.py` with `sage.all` support or `sage worker.py`).
  - Stdio duplex communication with JSONL framing.
- Tokio-based process manager:
  - Spawn worker in its own process group (`setsid`).
  - Capture real worker PID from ready envelope (`{"op": "ready", "pid": ...}`).
  - Non-blocking line readers on stdout and stderr.
  - Request wire queue with unique request IDs (`req-1`, etc.).
  - Cancellation / Interrupt: Send `SIGINT` to worker PID for swift cysignals abortion without killing worker; escalate to `SIGKILL` on process group if timeout occurs.
- Linux Sage Discovery:
  - Search PATH, `/usr/bin/sage`, `/usr/local/bin/sage`, and test `python3 -c "import sage.all"`.
- Unit and integration tests against live `worker.py`.
- **Gate:** Successfully spawn worker, execute `var('x'); factor(x^2 - 1)`, receive LaTeX & symbolic result, test SIGINT interruption on long-running task.

### Phase 3: `casette-gui` Shell, Tape View & Math Rendering
- Setup `eframe` desktop application with custom dark theme, typography, and cassette-styled visual accents.
- Implement `MathRenderer` trait:
  - Cache rendered textures keyed by LaTeX string.
  - Render LaTeX to vector/pixmap and upload to `egui::TextureHandle`.
  - Graceful fallback to `plain` text / Unicode math representation when math compilation fails.
  - Display plot artifacts (PNG/SVG) returned by worker.
- Implement the **Session Tape** view:
  - Virtualized scrollable list of tape entries.
  - Row badge `#1`, `#2`, etc.
  - Display user input, compiled Sage code, and rendered math output.
  - Row action menus: Copy Plain, Copy LaTeX, Copy Sage, Differentiate, Integrate, Simplify, Delete, Re-evaluate.

### Phase 4: Input Pane & Real-Time Lowering
- Multi-line input text editor with custom keybindings:
  - `Enter` or `Shift+Enter` configuration for submission.
  - `Escape` dismissals.
  - History up/down traversal.
- Live "Generated Sage" preview bar:
  - Runs `friendly-compiler` synchronously on every input change.
  - Shows compiled Sage code and required variables live before submission.
- Formula Autocomplete / Chip bar:
  - Quick insertion buttons for common mathematical templates (`factor(...)`, `diff(...)`, `integral(...)`, matrix builder).
  - Status indicators (Worker Ready / Busy / Evaluating / Error).
  - Cancel button active during long evaluations.

### Phase 5: Sidebar, Serde Persistence & Doctor
- Tabbed Sidebar (`egui::SidePanel`):
  - **Symbols Tab**: Query `op: "symbols"` from worker to display live bound variables, functions, and matrices in the Sage session.
  - **History Tab**: Searchable history of submitted calculations.
  - **Inspector Tab**: Deep metadata on the selected row (exact vs approx, dimension, execution time).
- Session Persistence:
  - Save and load sessions to clean JSON format using `serde`.
  - Export session to Markdown or LaTeX document.
- In-App Sage Doctor:
  - Health check dialog verifying Python, SageMath version, and LaTeX dependencies.
  - Configurable Sage binary path.

### Phase 6: Swift Code Removal & Final Linux Polish
- Remove all legacy Swift code: `Package.swift`, `Sources/`, `Tests/`, `v0/04*`, `build.sh`, etc.
- Retain only `v0/01-worker-protocol/worker.py` and documentation.
- Package for Linux:
  - `Makefile` or `cargo-dist` / `.desktop` file integration.
  - Final documentation update in `PLAN.md` and `PROGRESS.md`.
