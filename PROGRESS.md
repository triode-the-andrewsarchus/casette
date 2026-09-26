# Progress Log

## Summary

Casette is a native macOS SageMath-backed calculator with a persistent session tape. The project has completed the V0 proof sequence and V1.1 through V1.10 in the app: the SwiftPM app bundle builds locally, talks to a real Sage worker, renders math and plots, supports friendly input, exact/numeric controls, sidebar workflows, persistence/restore/replay/crash recovery, and now an in-app Sage Doctor for discovery and diagnostics.

Current gates are green: `swift test` (**642/642**, 97 suites), `make check`,
`make build`, and the worker envelope harness (**110/110**) passed for the
latest matrix-inspector work. Detailed historical notes live in [`progress/`](progress/),
newest first. Hard-won bug lessons live in [`PROBLEMS.md`](PROBLEMS.md).

**Rust & egui Port Progress (Phases 0 through 5 Complete):**
- **Phase 0:** Root Cargo workspace scaffolding (`friendly-compiler`, `casette-kernel`, `casette-gui`) configured with Linux target dependencies.
- **Phase 1:** Complete pure Rust port of `FriendlyCompiler` with 100% test parity across all compiler and IR lowerings (`cargo test` passes all 21 test suites).
- **Phase 2:** Complete Tokio-based `casette-kernel` crate:
  - Linux Sage environment discovery via `SageDiscovery` probing `python3 -c "import sage.all, sage.version..."` and candidate binary paths.
  - JSONL protocol types (`WorkerRequest`, `WorkerResponse`, `ArtifactPayload`, `TruncationPayload`, `MatrixPropertyPayload`, `ErrorPayload`, `SymbolPayload`) with full serde support and RFC 1574 rustdoc.
  - Asynchronous `SageKernel` client managing `worker.py` in its own process group (`setsid`).
  - Real worker PID capture from ready handshake envelope.
  - Fast interrupt delivery via `SIGINT` to worker PID for swift cysignals/Sage computation cancellation with zero worker corruption; clean session restart on stuck processes.
  - Automatic boot prelude execution (`var('u, v, w, x, y, z, ...')`) pre-populating standard calculator variables.
  - Verified with comprehensive live integration tests (`kernel_integration_tests.rs`) covering eval, ping, symbols, precision config, and SIGINT cancellation.
- **Phase 3:** `casette-gui` Core Shell & Math/Plot Rendering:
  - `theme.rs`: Refined dark theme adhering to typography scale and visual hierarchy with clean cassette accents.
  - `render/texture_cache.rs`: Vector SVG rendering with `resvg`/`usvg`/`tiny-skia` and raster PNG artifact loading into cached `egui::TextureHandle`s.
  - `render/math_view.rs`: Numbers/macOS-styled interactive matrix grid rendering (`[` `]`, padded cells, zero deemphasis), plot image renditions, error diagnostic cards, and row action menus (Copy Plain, Copy LaTeX, Differentiate, Integrate, Simplify).
  - `views/tape_view.rs`: Virtualized scrollable session tape with row number badges (`#1`), execution time pills, live state tracking, input expression rows, and compiled Sage indicators.
- **Phase 4:** Input Pane & Live Lowering:
  - `views/input_view.rs`: Multi-line editor with history navigation (Arrow Up/Down, Esc to clear, Enter to evaluate).
  - Synchronous "Generated Sage" live preview bar evaluating `friendly_compiler::compile` on every keystroke with auto-variable chips.
  - One-click formula chip templates (`factor`, `expand`, `diff`, `integral`, `solve`, `matrix`, `plot`, `taylor`, `sum`).
  - Prominent Cancel button wired to `SIGINT` interruption during long-running evaluations.
- **Phase 5:** Sidebar, Persistence & Doctor:
  - `views/sidebar_view.rs`: Tabbed sidebar with Symbols introspection (name/kind/summary table, filter, and hidden boot variables filter), History replay list, Inspector (selected row deep metadata, matrix properties), and Sage Doctor.
  - `persistence.rs`: Clean-sheet JSON session serialization and Markdown notebook export.
  - All 32 workspace tests passing cleanly, 0 warnings on `cargo clippy`.
- **Phase 6 (Post-Port Polish & Tape Reference Pipeline):**
  - **Bug 1 (Input Focus Loss):** Fixed issue where typing a command word like `"integrate"` caused the `TextEdit` editor to drop focus. Condition was caused by egui dynamic ID generation when the live lowering preview conditionally rendered widgets before an un-salted `TextEdit`. Resolved by assigning a stable `.id_salt("casette_main_input_editor")` and explicitly preserving focus across template clicks and clears.
  - **Bug 2 (Tape References `#ROW` & Plotting):** Implemented `TapeReferenceTable` in `friendly-compiler` and GUI evaluation pipeline:
    - Pre-compilation expansion of `#ROW` tokens outside quotes to `__casette_tape_refs[ROW]`, with validation against the 20-entry sliding reference window and descriptive errors for unavailable references.
    - Excluded `__casette_tape_refs` and private dunder identifiers from `required_variables` in `free_variables` inference so `var('__casette_tape_refs')` is never emitted.
    - Added automatic worker registration of successful row expressions to `__casette_tape_refs[ROW] = (<expr>)` and prune retired entries outside the 20-entry window.
    - Commands like `plot #1, x=-10..10` now correctly resolve row references into Sage `Graphics` objects without Python comment collisions.
    - Added automated unit and integration tests in `friendly_compiler_tests.rs` and `gui_tests.rs` verifying tape expansion, error row skipping, assignment code generation, and plot lowering. All 38 workspace tests green with 0 clippy warnings.
  - **Bug 3 (LaTeX Mathematical Typography Rendering):** Fixed math expression display to match original macOS Casette TeX typography:
    - In `v0/01-worker-protocol/worker.py`: Added `_render_math_artifact(latex_str)` leveraging matplotlib's built-in Computer Modern (`cm`) mathtext engine (`FontProperties(size=20)`, `#F0F6FC` primary text color matching dark theme, transparent background patch). Emitted as `{"type": "math", "format": "svg", ...}` in result envelopes whenever Sage generates a valid `latex` representation.
    - In `crates/casette-gui/src/render/texture_cache.rs`: Upgraded SVG rasterization to 2.0x scale for crisp HiDPI/Retina display; added `get_or_render_latex(&mut self, ctx: &Context, latex: &str)` caching fallback to `/tmp/casette_math_<hash>.svg` for restored or offline sessions.
    - In `crates/casette-gui/src/render/math_view.rs`: Refactored result routing so plot cards only render `artifact.r#type == "image"`, matrix views render grids, and mathematical results route to `render_primary_math` displaying vector math textures at natural proportional size with graceful fallback to `render_primary_text`.
    - Added automated GUI test `test_math_rendering_and_texture_cache` and kernel integration test `test_kernel_boot_eval_ping` asserting math SVG generation and valid on-disk artifacts. All 39 tests pass with 0 warnings.
  - **Phase 7 (Physical Units System & Natural Unit Parser):** Implemented natural dimensional physical units:
    - Preloaded 69 standard SI and Imperial units in `v0/01-worker-protocol/worker.py` (`mi`, `km`, `ft`, `s`, `hr`, `kg`, `mph`, etc.) and added `__casette_convert` for automatic dimensionless cancellation (e.g. `50*km / (32*mi)` -> `390625/402336`), compatible unit sum unification, and explicit conversion clauses (`to miles`, `in mph`).
    - Added `units.rs` and `unit_parser.rs` in `friendly-compiler` powered by `logos`:
      - Disambiguates units from unknown algebraic variables in `free_variables` so `solve 32*mi*x^2 = 52*mi^2` and `solve 32 mi * x^2 = 52 mi^2` automatically isolate `x` without ambiguity.
      - Desugars natural unit juxtaposition for both numbers (`32 mi`, `52 mi^2`, `9.8 m/s^2`) and algebraic variables / parenthesized terms (`x sec`, `t hr`, `(x + 1) hr`) into explicit parenthesized expressions (`(32*mi)`, `(x*sec)`, `(x + 1)*(hr)`), eliminating Python `SyntaxError: Invalid Syntax` on multi-word variable-unit terms.
      - Fixed division precedence so expressions with units in denominators (`100 km / 2 hr in mph` or `/ 2*hr`) parenthesize as `/(2*hr)` rather than inverting into multiplication `((100*km) / 2) * hr = 50 * km * hr`, eliminating Sage `ValueError: Incompatible units`.
      - Unwraps redundant single-argument `__casette_convert(...)` calls during conversion clause compilation, ensuring clean single-layer `__casette_convert((100*km) / (2*hr), mph)` lowering.
      - Added sequence and relation recursion in worker runtime `__casette_convert`: equations and solve results involving units automatically evaluate compatible units (e.g. `solve 3 hr + x sec = 12 days` simplifies to `[x == 1026000*second]`).
      - In `solve` equations where the unknown is multiplied by a target unit (e.g. `solve 3mi + x*ft = 7km` yielding quotient `(7*km - 3*mi)/ft`), `__casette_convert` detects relations whose RHS is a dimensional quotient and converts to the target denominator unit (`[x == 2714960/381*foot]`).
      - Implemented `_napprox_object` and expanded `_napprox_string` and `_build_envelope` in `worker.py` to evaluate numerical approximations on lists, tuples, and relational expressions (`is_relational()`). When numeric mode is toggled, solve equations format their numerical approximation as the primary display (`[x == 7125.879265*foot]`) while retaining the exact fraction in `exact_value`.
      - Parses conversion clauses (`... to <unit>` / `... in <unit>`).
  - **Phase 8 (Full SageMath Unit Catalog & Ohm's Law Dimensional Conversions):**
    - Dynamically registered all 267+ base and derived SageMath units across 45 physical categories from `sage.symbolic.units.unitdict`, including automatic plural generation, SI prefixes, and custom abbreviations totaling 785 recognized unit symbols.
    - Added all SI base units (Ampere: `A`, `Amp`, `mA`, `uA`, `kA`; Kelvin: `K`, `Kelvin`, `degC`, `degF`; Candela: `cd`, `lm`, `lx`; Mole: `mol`, `mmol`; Second: `s`, `ms`, `us`, `ns`, `min`, `hr`, `day`, `wk`, `yr`; Meter: `m`, `km`, `cm`, `mm`, `in`, `ft`, `yd`, `mi`, `ly`, `au`; Kilogram: `kg`, `g`, `mg`, `lb`, `oz`, `ton`).
    - Added full volume unit catalog (`L`, `liter`, `mL`, `uL`, `dL`, `cL`, `gal`, `gallon`, `qt`, `pt`, `cup`, `floz`, `tbsp`, `tsp`).
    - Preserved exact Rational arithmetic for all unit scales using Sage's `QQ` (`sage.rings.rational.Rational`), preventing floating-point inaccuracies in ratios and solve expressions (e.g. `50*km / (32*mi)` simplifies to exact rational `390625/402336`).
    - Added `_DERIVED_SI_TARGETS` and automatic compound unit reduction in `__casette_convert`: expressions with compound dimensions simplify directly to derived SI units (`12V / 3A` -> `4*ohm`, `120V * 15A` -> `1800*watt`, `2A * 50ohm` -> `100*volt`).
    - Cleanly separated assignable LHS identifiers (`is_assignable_variable`) from free variables (`is_plausible_variable`): assignments to single capital letters like `A = matrix(...)` or `V = ...` succeed without being blocked by unit checks, while algebraic expressions recognize `V`, `A`, `K`, `W`, etc. as physical units rather than free unknown variables.
    - Added comprehensive unit tests in `friendly-compiler` (`test_ohms_law_and_electric_units`, `test_volume_and_temperature_units`) and live kernel integration tests in `casette-kernel` (`test_kernel_expanded_units_and_ohms_law`). All 59 workspace tests passing.
    - Added comprehensive unit tests in `friendly_compiler_tests.rs` and integration tests in `kernel_integration_tests.rs`. All 48 workspace tests green with 0 clippy warnings.
  - **Phase 8 (Action Bar Ergonomics, Hit-Testing & Decimal Approximation):**
    - **Issue 1 (Action Bar Visual Styling):** Replaced unstyled flat text buttons with macOS-styled pill buttons featuring rounded corners (`CornerRadius::same(5)`), subtle borders (`BORDER_SUBTLE`), dark container fill (`#26262C`), proportional typography (11.5pt), semantic icon glyphs (`📋 Copy`, `TeX Copy LaTeX`, `≈ Approximate`, `∂/∂x Differentiate`, `∫ Integrate`, `✦ Simplify`), and interactive hover tooltips.
    - **Issue 2 (Card Hit-Testing / Unclickable Buttons):** Discovered root cause of unclickable buttons in `tape_view.rs`: `card_response.response.interact(egui::Sense::click())` registered a broad hit-testing rectangle over the entire card bounding box *after* the inner widgets had rendered, causing egui's hit-tester to prioritize the card rect and swallow all child button clicks. Replaced with non-intrusive pointer state inspection (`ui.rect_contains_pointer(card_rect) && ui.input(|i| i.pointer.primary_clicked())`), completely restoring direct clickability to all action buttons while preserving card selection in the inspector.
    - **Issue 3 (Clipboard Feedback):** Added dynamic temporary visual feedback for "Copy" and "Copy LaTeX" (`✓ Copied`, `✓ Copied LaTeX` in accent green) stored per-row in egui temporary state.
    - **Feature (Approximate Button & Decimal Conversion):**
      - Added "Approximate" button (`≈ Approximate`) to result action bars for rational numbers, fractions, constants, and unit expressions. Emits `approx #<row>` to compute high-precision decimal values.
      - Fixed `worker.py` and `friendly-compiler` keyword collision between Sage's built-in `N` / `numerical_approx` and unit `"N"` (Newton): desugared standalone `"N"` token to `"newton"` in parser and registered `_casette_approx` on `N`, `approx`, and `numerical_approx`.
      - Enhanced `_casette_approx` and `_napprox_string` in `worker.py` to evaluate numerical approximations on dimensional expressions by approximating numerical coefficients while retaining physical units (e.g. `100*mi / (3*hr)` -> `33.3333333333333*mile/hour`).
      - Wired `app.rs` action submission to propagate `is_numeric` flag when approx is triggered.
      - Added automated test `test_action_buttons_and_approx_compilation` in `gui_tests.rs`. All 46 tests green, 0 clippy warnings.
  - **Phase 9 (Problems Audit & Obsolete Swift Documentation Cleanup):**
    - Audited all 28 entries in the `problems/` directory against the Rust codebase (`casette-rs`), Tokio process manager, Sage `worker.py`, and `friendly-compiler`.
    - **Retained (16 applicable):**
      - `001`: Session persistence schema refusals, non-persisted transient restore UI state, in-flight row interruption.
      - `002`: Restart re-init chaining on serial kernel queue ahead of subsequent evaluations.
      - `009`: Friendly compiler newline flattening vs multiline raw Sage preserving.
      - `010`: POSIX child signal inheritance, `SIGINT` delivery, and CPython/cysignals signal masks.
      - `011`: Session persistence with missing temporary plot artifacts and schema version peeking.
      - `013`: Symbolic Ring exactness (`is_exact() == False`) and tree-walking symbolic expressions.
      - `014`: Numeric precision clamping to native `prec()` to prevent `.n(bits)` exceptions.
      - `015`: Friendly compiler space boundary bypass and symbolic definite integrals.
      - `016`: Symbol table baseline diffing by object identity (accounting for Sage's `n`/`N`/`i`).
      - `020`: Classifying Sage results across quadratic number fields, sequence generics, and booleans.
      - `021`: Numeric approximation preserving native precision without blind `digits=` calls.
      - `022`: Interrupting Sage with cysignals owning `SIGINT` for prompt C computation aborts.
      - `023`: Worker restart generation isolation for reader queues and EOF events.
      - `024`: `sage -python` bash wrapper process group targeting for clean termination without orphans.
      - `025`: Protocol stdout separation using private dup'd fds and OS-level fd redirection.
      - `026`: Sage value-echo parsing preparsed code via `ast` and suppressing `None`.
    - **Removed (12 obsolete Swift/SwiftUI/AppKit/SwiftPM entries):**
      - `003`: SwiftUI `AttributeGraph` spin on `NSView` self-invalidation during graph updates.
      - `004`: SwiftUI `.windowResizability(.contentMinSize)` min-height explosion with wrapping text.
      - `005`: Parallel `swift-testing` test runner environment races with `setenv`.
      - `006`: SwiftMath `MTMathUILabel` NSView layout performance in `LazyVStack`.
      - `007`: macOS SwiftUI `.popover` key window focus and Cocoa `cancelOperation:` interception.
      - `008`: macOS automation harness event tap consuming keyCode 53 (Esc).
      - `012`: Swift `Foundation.Process` lacking `setsid` (Rust `tokio::process` handles this natively).
      - `017`: macOS AppKit `NSImage` rendering matplotlib SVG `<use>` elements as black blobs.
      - `018`: LaTeXSwiftUI / MathJax in JavaScriptCore failing braced scripts.
      - `019`: SwiftMath missing `array` environment and macOS `NSView.fittingSize` vs `intrinsicContentSize`.
      - `027`: SwiftMath `MTTypesetter` layout assertions on nested `\left...\right` groups.
      - `028`: SwiftPM `Bundle.module` resource bundle patching for hand-assembled `.app`s.
    - Updated master `PROBLEMS.md` index to cleanly reflect active lessons.
  - **Phase 10 (One-by-One Problem Resolution, Automated Testing & Markdown Retirement):**
    - Methodically verified and addressed all 16 applicable problems commit-by-commit:
      - **Problem 001 (`20fd4a9`):** Implemented schema versioning (`CURRENT_SCHEMA_VERSION`), version peek before decoding, future version refusal (`bail!`), in-flight evaluation interruption on restore, and clearing transient UI live state. Verified in `casette-gui/tests/gui_tests.rs`.
      - **Problem 002 (`c73426f`):** Enforced serial order on `WorkerActor` loop so `boot_prelude` (`var('u, v, w, x, y, z...')`) always finishes before subsequent user evaluations. Verified in `kernel_integration_tests.rs`.
      - **Problem 009 (`a5dbc5c`):** Preserved multi-line Sage code newlines without single-line compiler flattening when input contains `\n`. Verified in `gui_tests.rs`.
      - **Problem 010 (`e8bf937`):** Reset child process signal dispositions (`SigSet::empty()`, `SIG_DFL`) in `cmd.pre_exec` so child workers don't inherit ignored SIGINT from parent.
      - **Problem 011 (`be7a409`):** Rendered graceful placeholder cards for missing or expired temporary plot artifacts in `render_plot_card`. Verified in `gui_tests.rs`.
      - **Problem 013 (`c2abf8f`):** Enforced Sage Symbolic Ring exactness distinction in `worker.py` (`_is_exact`, `_symbolic_has_inexact_atom`), correctly handling exact symbolic constants like `sin(1)` vs floating approximations.
      - **Problem 014 (`e4da26c`):** Clamped `.n(bits)` approximation requests in `worker.py` to concrete objects' native `value.prec()`, preventing Sage bit-overflow exceptions.
      - **Problem 015 (`f7b36e1`):** Verified friendly compiler whitespace boundary rules on symbol bypass and symbolic classification of definite integrals in `friendly_compiler_tests.rs`.
      - **Problem 016 (`a8d8584`):** Diffed symbol table namespace against baseline by object identity (`live_value is not baseline_obj`), allowing shadowed Sage builtins (`n = 104729`, `N`, `i`) to be reported. Verified in `kernel_integration_tests.rs`.
      - **Problem 020 (`761e9d2`):** Corrected Sage type classification in `_classify` for Gaussian literals (`QQ[i]`), `Sequence_generic` (`solve`), and `bool` preceding `int`. Verified in `kernel_integration_tests.rs`.
      - **Problem 021 (`24d565b`):** Ensured numeric approximation respects native precision on concrete reals/complexes without blind `digits=` calls, and gated `approx` to scalar types. Verified in `kernel_integration_tests.rs`.
      - **Problem 022 (`273a727`):** Let cysignals own SIGINT by installing worker handlers prior to Sage imports, and directed SIGINT to real worker PID for prompt aborts of C/GMP loops. Verified in `kernel_integration_tests.rs`.
      - **Problem 023 (`7c1040e`):** Provided each worker generation its own isolated `WorkerProcess` and stream readers, eliminating restart EOF races.
      - **Problem 024 (`c5030c9`):** Spawned child processes with `process_group(0)` and signaled the entire process group (`-pid`) in `kill_and_reap` to ensure wrapper scripts (e.g. `sage -python`) and child Python workers are cleanly terminated without leaks.
      - **Problem 025 (`a6d4a39`):** Maintained private dup'd file descriptors (`os.dup(1)`, `os.dup(2)`) for JSONL protocol communication, redirecting both Python `sys.stdout` and OS-level fds 1/2 to capture buffers without stream desynchronization. Verified in `kernel_integration_tests.rs`.
      - **Problem 026 (`2888fb4`):** Evaluated preparsed AST and suppressed `None` returns in `_eval_code` to prevent bare prints or assignments from returning `None` values, and verified unordered factor terms. Verified in `kernel_integration_tests.rs`.
    - Removed all 28 individual problem markdown files from `problems/` and updated master `PROBLEMS.md` and `PLAN.md`.
  - **Phase 11 (Interactive 2D Plotting with SageMath Backend):**
    - Created branch `feat/sage-interactive-plotting` branched cleanly off `feat/units-and-parser` (`5579fb5`), git-ignoring `vendor/` and bytecode artifacts.
    - Preserved SageMath's canonical calculus and algebra engine (`integrate(1/(1-x^2), x)` natively yields $\frac{1}{2}\log(x+1) - \frac{1}{2}\log(x-1)$).
    - Extended `v0/01-worker-protocol/worker.py`:
      - Implemented `_extract_plot_data` extracting sampled curves (`p.xdata`, `p.ydata`), bounding ranges, colors, and labels from Sage `Graphics` objects.
      - Attached structured `plot_data` to `WorkerResponse` while continuing to save SVG and PNG image artifacts for notebook export and session persistence.
    - Extended `crates/casette-kernel`:
      - Added `PlotDataPayload`, `PlotSeriesPayload`, `LocalExtremum`, and `ExtremumKind` to `protocol.rs`.
      - Implemented `compute_local_extrema` analyzing discrete difference signs across sampled curve points to detect peaks and valleys.
      - Enriched responses in `read_response` to populate extrema automatically.
      - Added integration test `test_kernel_interactive_plot_data` in `kernel_integration_tests.rs`.
    - Enhanced `crates/friendly-compiler`:
      - Implemented `split_trailing_range` in `compiler.rs` to support comma-free plot commands (e.g. `plot sin(x) x=-pi..pi` and `plot sin(x) -pi..pi`).
      - Updated `PlotFormulaIR::parse` to parse comma-free ranges into formula chips.
      - Preserved `#ROW` tape reference compilation (`plot #1, x=-10..10` -> `plot(__casette_tape_refs[1], (x, -10, 10))`).
    - Enhanced `crates/casette-gui`:
      - Added `egui_plot` 0.31 to workspace and `casette-gui`.
      - Implemented `crates/casette-gui/src/render/interactive_plot.rs` featuring panning, zooming, 18px magnetic extrema snapping (`🧲 Snap Extrema: ON/OFF`), cursor crosshair inspection, and "↺ Reset" button.
      - Sliced curve points into contiguous finite segments (`x.is_finite() && y.is_finite()`) to prevent `epaint` coarse tessellation culling from dropping lines on discontinuous or singular functions.
      - Excluded plot commands and kind `"plot"` from `TapeRow::reusable_expression` and `extract_reusable_expression`.
      - Routed interactive plots in `crates/casette-gui/src/render/math_view.rs`.
    - Documented subsystem in `plans/INTERACTIVE-PLOTTING.md` and indexed in `PLAN.md`.
    - All 45 tests green with 0 clippy warnings; release binary built.
  - **Phase 12 (Resolution of 7 User-Reported Issues — One Commit Per Issue):**
    - **Issue 1 (`6a746e5` - Symbols Tab Plumbing Leak):**
      - Filtered out `name.startswith("_")` in `worker.py` (`_handle_symbols`) and hardened `crates/casette-gui/src/views/sidebar_view.rs` to ignore symbols starting with `_`, completely eliminating internal `__casette_tape_refs` leaks.
      - Added clear empty state guidance explaining how to declare variables (`a = 5`).
    - **Issue 2 (`41804f3` - Automatic Session & History Persistence):**
      - Added `default_session_path()` respecting `$CASETTE_CONFIG_DIR`, `$XDG_CONFIG_HOME`, and `$HOME/.config/casette/sessions/last-session.json`.
      - Extended `SessionEnvelope` to serialize and restore calculation history (`state.history`).
      - Wired automatic loading in `CasetteApp::new` and auto-save on evaluation completion, tape clear, `App::save`, and application drop.
      - Added automated tests in `gui_tests.rs`.
    - **Issue 3 (`05ebee5` - Ineffective Approximate Toggle):**
      - Root cause: In force-numeric mode (`numeric=True`), `worker.py` failed to update `latex_str = approx_str`, causing the worker to generate a math SVG artifact of the exact symbolic expression which the GUI then rendered.
      - Updated `worker.py` to set `latex_str = approx_str` and suppress exact math SVG artifacts in numeric mode.
      - Updated `math_view.rs` to render primary approximate results with a prominent `≈` prefix in numeric amber and display the exact fallback (`exact: <expr>`) when available.
      - Added unit tests in `gui_tests.rs` and live integration test in `kernel_integration_tests.rs`.
    - **Issue 4 (`1a58dfd` - Connected Sage Doctor Header Button):**
      - Connected the header Doctor button to an interactive diagnostics modal window (`Window::new("🩺 Sage Doctor")`) displaying live SageMath version, worker PID, color-coded status badge, worker restart action, and a button to focus the sidebar Doctor tab.
      - Styled the toolbar Doctor button with active selection feedback and hover tooltips.
    - **Issue 5 (`901cc64` - Clear Tape Button Visual Affordance):**
      - Replaced flat unstyled `.frame(false)` text with a modern macOS-styled toolbar button featuring rounded card fill (`Colors::CARD_BG`), subtle border stroke (`Colors::BORDER_SUBTLE`), corner radius (5), trash icon (`🗑 Clear Tape`), readable text color, and hover feedback.
    - **Issue 6 (`eb6aeee` - High-Contrast Plot Curve Palette on Dark Theme):**
      - Root cause: SageMath's default curve option `opts["rgbcolor"]` is `(0.0, 0.0, 0.8)` (`[0, 0, 204]`), rendering as dark navy blue on dark backgrounds.
      - Updated `worker.py` and `interactive_plot.rs` to detect and replace Sage's default navy blue or low-luminance colors with a bright, vibrant dark-mode palette (`#38BDF8` sky blue, `#34D399` emerald green, `#FBBF24` amber, `#C084FC` lilac purple).
      - Increased curve stroke width to `2.5px` for crisp visibility.
    - **Issue 7 (`bb2bc34` - Intuitive Plot Navigation Controls):**
      - Enabled `allow_scroll(true)` for mouse-wheel and trackpad zoom centered at the cursor.
      - Enabled `allow_double_click_reset(true)` for quick reset on canvas double-click.
      - Added on-screen `[ ➕ ]` (zoom in 1.3x) and `[ ➖ ]` (zoom out 0.77x) toolbar buttons alongside `[ ↺ Reset ]`.
      - Added explicit gesture hints in the plot card header: `"Pan: drag · Zoom: scroll/[+][−] · Reset: double-click"`.
    - **Issue 8 (Session Restore Row Numbering & Tape Reference Desync):**
      - Root cause: When `last-session.json` was restored, `SessionState.next_row_number` advanced to `N + 1`, but the Tokio actor in `main.rs` maintained an independent `row_counter` initialized to `1`. When evaluating new expressions, `app.rs` allocated row `N + 1` in the UI as evaluating, while `main.rs` processed it as row `1` and emitted `AsyncMessage::EvalFinished { row_number: 1, ... }`. The UI updated row 1 (the restored row) and left row `N + 1` spinning indefinitely. Restarting the worker also reset `row_counter = 1`, failing to recover.
      - Fixed `GuiAction::Evaluate` to pass the explicit `row_number` allocated by `SessionState`.
      - Added `GuiAction::SyncTapeRefs` dispatched on `WorkerReady` to synchronize all restored tape references (`__casette_tape_refs[i] = (...)`) into Sage, ensuring `#1`, `#2`, etc. are immediately usable in subsequent evaluations.
      - Added automated test `test_restored_session_row_number_continuity` in `gui_tests.rs`. All workspace tests green (47/47).
    - **Issue 9 (Implicit Multiplication in Sage Preparser):**
      - Root cause: Input forms like `solve x^2 + 5x + 6 = 0` and `5x` raised `SyntaxError: invalid decimal literal` in Python because Sage's preparser disabled implicit multiplication by default (`implicit_multiplication` set to level 0).
      - Enabled `from sage.repl.preparse import implicit_multiplication; implicit_multiplication(True)` in `worker.py`. Expressions like `5x`, `2pi`, and `solve x^2 + 5x + 6 = 0` now preparse cleanly to `Integer(5)*x` and solve properly.
      - Added automated integration test in `kernel_integration_tests.rs`.
    - **Issue 10 (Plot Scroll Isolation from Outer Tape View):**
      - Root cause: `egui_plot::Plot` did not consume or clear `smooth_scroll_delta` after handling scroll interactions, allowing mouse-wheel scroll events over the plot canvas to leak into the outer tape `ScrollArea`, causing both the plot and the whole tape to scroll simultaneously.
      - Intercepted scroll events when the pointer is over the plot canvas (`plot_resp.response.contains_pointer() || plot_resp.response.hovered()`) and cleared `smooth_scroll_delta = Vec2::ZERO` and `raw_scroll_delta = Vec2::ZERO`. Scrolling over a plot now exclusively navigates the plot.
    - **Issue 11 (Zero-Crossing / Root Detection and Snapping):**
      - Added `ExtremumKind::ZeroCrossing` to `protocol.rs` and upgraded `compute_local_extrema` to detect zero crossings via linear interpolation across sign changes ($(y_{i-1} < 0 \land y_i > 0)$ or $(y_{i-1} > 0 \land y_i < 0)$) and exact zeros, with asymptote guarding ($|y_i - y_{i-1}| < 50$) and deduplication.
      - Rendered distinct emerald circle markers for zero-crossings (`Colors::EXACT_GREEN`) and added card badge reporting (`⨁ Zero / Root: x = ..., y = 0`).
      - Updated snap toggle button to `"🧲 Snap (Extrema & Zeros): ON/OFF"`.
      - Added unit test `test_compute_local_extrema_and_zero_crossings` in `casette-kernel`.
    - **Issue 12 (Snapped Coordinate Readout in Bottom Left):**
      - Replaced `CoordinatesFormatter::default()` with custom `CoordinatesFormatter::new` backed by a shared cell tracking the active snapped point.
      - When snapped, the bottom-left coordinate display renders the exact **snapped value** (`x: ..., y: ... (Zero/Max/Min)`), rather than the raw hovering cursor coordinate.
      - All workspace tests green (48/48); release binary compiled and verified.

**Latest matrix-inspector work:** Matrix result envelopes now carry additive
`matrix_properties` metadata from Sage's no-argument `is_*()` methods. Swift
maps that to persisted `matrixProperties`, and the Inspector's (i) tab renders
a compact **Matrix Properties** section: true facts are visible immediately,
while false and unavailable facts sit in disclosure groups so the sidebar stays
scan-friendly but still exposes the full Sage property set. The worker protocol
and session-format docs record the new field; focused Swift tests and the V0.3
worker envelope harness pin the mapping. Live Computer Use verification on
`build/Casette.app` confirmed that evaluating `matrix([[1,2],[3,4]])` selects
the fresh row and shows the properties section at the top of the Inspector tab.

**Latest SwiftMath launch-crash hardening:** A user crash report from
2026-06-13 showed `Bundle.module` trapping while `MTMathUILabel` initialized
its default Latin Modern font. The build now copies SwiftMath's generated
`SwiftMath_SwiftMath.bundle` into `Contents/Resources`, verifies the generated
SwiftMath accessor was patched to search that location, and fails the build if
`latinmodern-math.otf` is missing from the copied bundle. This keeps
copied/distributed app bundles from depending on the developer `.build`
resource fallback.

**Latest matrix-table work:** Abbreviated matrix rows now offer a subtle
`tablecells` pop-out button next to Copy. The affordance appears exactly when
the matrix card's LaTeX preview uses `\cdots`/`\vdots`; opening it asks Sage for
the stored tape value via `__casette_tape_refs[#]`, then fetches the hidden
matrix-table JSON from Sage in chunks so 100x100 tables do not hit the worker's
8 KiB plain-output cap. The full table's cells come from Sage rather than from
parsing the abbreviated display. The pop-out is a native resizable window with
two-axis scrolling, hover row/column feedback, very deemphasized zeros, padded
column headers, and Esc-to-close. Restored or older-than-reference-window rows
do not show the table affordance until their value is live in Sage again.
Follow-up fixes: the table affordance is now persistent instead of hover-only,
row selection no longer swallows embedded controls, the 100x100 table body uses
lazy row rendering with a compact accessibility tree to avoid beachballing, and
the visible app was verified by evaluating `random_matrix(QQ, 100, 100)`,
clicking the live row's pop-out, opening `Matrix #20`, observing the app settle
to 0.0% CPU, and closing it with Esc. Validation: `make check`, focused
matrix-table tests, `make build`, plus the earlier full `swift test` **640/640**
and worker envelope harness **109/109**.

**Latest crash fix:** SwiftMath parser success is no longer treated as enough
to render LaTeX. A crash report showed `MTTypesetter.getInterElementSpace`
asserting while SwiftUI measured an `MTMathUILabel`; the stack passed through
nested `makeLeftRight` calls, matching nested `\left...\right` delimiter groups.
Casette now detects nested delimiter groups after Sage normalization and falls
back to the worker's plain text before SwiftMath can measure them. Sequential
delimiter groups and normalized Sage matrices still render as math. Focused
rendering tests cover tuple-list basis LaTeX, nested braces, sequential groups,
and normalized matrices. Details:
[`problems/027-swiftmath-can-parse-nested-left-right-groups-then-assert.md`](problems/027-swiftmath-can-parse-nested-left-right-groups-then-assert.md).
Validation: `make check`, `swift test` **637/637**, and `make build` green.

**Latest vector-row crash fix:** Indexing a row out of a matrix (for example
`random_matrix(QQ, 7)[0]`, or the same through a tape reference) returns a Sage
vector whose default LaTeX is a flat tuple such as
`\left(0,\,1,\,2,\,3,\,4,\,5,\,6\right)`. SwiftMath can parse that shape but
assert while measuring it. Fresh worker envelopes now render standalone vectors
as one-row array matrices, abbreviated with `\cdots` after the first three
entries when the vector has more than five entries. Legacy persisted tuple
LaTeX is marked unsafe in `MathContent` and falls back to plain text.

**Latest help-system work:** Casette now ships a native Help window for the
Friendly Compiler mini-language. The Help menu has **Friendly Compiler
Language**, opening a separate searchable reference window that covers the
current command families and shows generated Sage for each example. The content
is code-native (`HelpReference`) so it packages reliably with the SwiftPM app
bundle, and `HelpReferenceTests` guard representative command coverage plus the
key boundary rules. Validation: `make check`, `swift test` **634/634**, and
`make build` green. Details: [`plans/HELP-SYSTEM.md`](plans/HELP-SYSTEM.md).

**Latest bug fix:** Sage help output no longer makes the app appear hung.
`help(x)` for a symbolic expression can emit hundreds of KB of pydoc text; the
worker now caps captured stdout/stderr at 32,768 characters and appends a visible
truncation note, so the existing stdout block can show Sage help text without
overwhelming SwiftUI. The stdout view also renders only a 16,384-character
preview for restored rows, so a pre-fix persisted help row cannot slow app
startup. The V0.1 worker harness now covers `help(x)` as bounded stdout.
Validation: worker harness **19/19**, focused stdout display tests, `make check`,
`swift test` **632/632**, and `make build` green.
Details: [`progress/000-2026-06-13-sage-help-stdout-cap.md`](progress/000-2026-06-13-sage-help-stdout-cap.md).

**Latest statistics work:** [plans/STAT.md](plans/STAT.md)'s first and second
batches are implemented, backed by [plans/PRELOAD.md](plans/PRELOAD.md)'s
hidden worker preloads. The canonical worker now installs normal, binomial,
Poisson, exponential, and uniform helper functions before the symbol-table
baseline, so raw Sage can call `normal_between(-1, 1)` and
`binomial_cdf(3, n=10, p=.5)` without those helpers appearing in Symbols.
Friendly compiler Batch G adds 22 distribution commands with compact formula
bars and readable generated Sage; `lambda=` lowers to Python-safe `lambda_=`,
and `min=`/`max=` lower to `low=`/`high=`. Validation: `swift test`
**630/630**, `make check`, `make build`, and the worker envelope harness
**97/97**.

**Latest layout work:** The main tape/input boundary and the sidebar's
top/content boundary are now resizable with app-owned persisted dimensions
(`inputPaneHeight`, `sidebarTopPaneHeight`) alongside the existing sidebar
visibility/tab layout keys. The reusable vertical split container clamps panes
so both sides remain usable and stores changes through `@AppStorage`. The input
pane layout was tightened after live review: the editor now owns the full pane
width, with key hints and exactness/status controls moved to a secondary row so
they do not force an internal editor scrollbar in the middle of the pane.

**Latest calculator-variable maintenance:** The Sage boot prelude now
predefines `u`, `v`, `w`, `x`, `y`, `z`, `x1`...`x9`, `y1`...`y9`, and `t`.
The prelude is generated from `ShellModel.bootVariableNames`, and boot/restart
tests assert against that single source so the Symbols sidebar contract stays
honest. The Symbols tab now hides untouched built-in boot variables by default,
with a persisted **Show Built-in Variables** checkbox for users who want the
full namespace visible; reassigned names such as `x = 5` still appear. The
toolbar's Clear Tape control is now an icon-only trash button, with a matching
menu command that opens the same confirmation dialog.

**Latest maintenance:** Result actions now include **Trig Simplify** for symbolic rows and three basis-producing matrix actions: **Column Space** (`column_space().basis()`), **Row Space** (`row_space().basis()`), and **Right Kernel Basis** (`right_kernel().basis()`). The existing **Simplify** action remains the full simplification path (`simplify_full()`), so no separate `simplify_full` action is exposed. The worker action vocabulary, Swift command mapping, placeholder data, and worker-protocol docs were updated together. A live crash from `right_kernel().basis()` exposed a SwiftMath layout assertion for Sage tuple-list LaTeX (`\left[\left(...\right)\right]`): legacy tuple-list basis LaTeX now falls back before layout, and fresh worker envelopes render Sage `Sequence_generic` values whose elements are vectors as labeled column-vector bases (`\mathcal{B} = \left\{\begin{pmatrix}...\end{pmatrix}\right\}`). Validation: `swift test` **610/610**, `make check`, `make build`, and `v0/03-result-envelope/harness.py` **97/97**. Computer Use live pass on `build/Casette.app`: matrix basis actions rendered one- and two-vector bases in the screenshot-style LaTeX shape, and `sin(x)^2 + cos(x)^2` followed by Trig Simplify produced `1` with no crash.

**Matrix action extension:** Matrix result actions now always include **Change
Ring to RDF** (`change_ring(RDF)`). Matrices whose base ring is already `RDF` or
`CDF` also include **SVD**; the Swift action evaluates
`__casette_svd_labeled((M).SVD())`, and the worker's hidden preload helper
renders Sage's returned `(U, S, V)` triple as labeled `U = ...`, `S = ...`, and
`V = ...` matrices in both plain text and LaTeX. The action vocabulary,
placeholder rows, worker-protocol docs, worker harness, Swift action tests, and
real-Sage rendering/sidebar journeys were updated together.

**Matrix display abbreviation:** Matrix LaTeX now abbreviates only when a matrix
has more than five rows or more than five columns. The display shows the first
three rows/columns and the final row/column, using `\cdots` for omitted columns
and `\vdots` for omitted rows, so large transformations stay readable on the
tape while the underlying Sage value remains unchanged.

**Previous maintenance:** Tape entries are now visibly numbered, and prompts can reuse the last 20 successful reusable tape expressions with `#ROW` syntax. Errors still occupy row numbers but are not valid references; missing/stale references surface as normal compile-preview errors. `#57` expands before friendly compilation to the private Sage dictionary lookup `__casette_tape_refs[57]`, and the worker dictionary is refreshed after successful evals and during Replay Session so restored sessions can rebuild reference state. The private dictionary is filtered out of the Symbols sidebar because it is app plumbing, not user-serviceable state. The app's default approximation precision is now 5 digits (configured at boot over the worker's native 10), and the precision menu includes 2- and 3-digit choices. Tests cover reference expansion, missing references, error-skipping with visible row numbers, the 20-entry window, replay rebuild, symbol filtering, default precision, and existing numeric/queue semantics; `swift test` is green at **302/302**.

**Formula autocomplete prototype:** The input pane now grows a compact
Numbers-style formula bar when the draft starts with `integral`/`integrate`.
The bar edits expression, variable, and the two optional definite bounds while
rewriting the source draft back to friendly input, not Sage. The shared IR is
`IntegralFormulaIR` in `FriendlyCompiler`; it preserves app-level source such
as `#14` tape references until `CompiledInput` expands them at the existing
compile boundary. The friendly compiler also accepts explicit indefinite
integrals as `integral expr, wrt var`. Focused tests cover `wrt`, IR rendering,
`#ROW` preservation/expansion, and model draft rewrites. `swift test` is green
at **305/305** and `make build` is green. The function chip is pinned outside
the argument lane, and the formula expansion now sits on its own full-width
lane below the input controls so the optional bounds are visible without
horizontal scrolling at the normal window size.

**Completion UI hitlist (2026-06-12, branch `fable/completion-hitlist`):** The
integral formula bar's architecture now covers **36 completions** across six
batches, all dispatched through one new seam: `FormulaIR` (FriendlyCompiler)
wraps each family's typed IR, `ShellModel.formulaIR`/`updateFormula` is the
single draft funnel, and `FormulaBarView` switches the hint lane. Every bar
follows the COMPLETION-UI.md Extension Rule — parse the draft, edit a typed
IR, render BACK to friendly input (never Sage), `#ROW` references preserved
verbatim until `CompiledInput`, optional args grouped as one semantic chip.
Implemented (trigger words): expand, factor, simplify, solve · derivative
(+optional order clause), limit (+optional left/right direction), taylor, sum,
product (new lowerings) · plot, parametric_plot, implicit_plot (new lowerings;
equation sugar `=`→`==`, bare expr → `== 0`) · matrix, vector, det/determinant,
inverse, transpose, rank, rref, eigenvalues, eigenvectors (matrix methods now
accept variables/expressions/tape refs: `det A` → `(A).det()`) · gradient,
jacobian, hessian, subs, numeric/approx/decimal, latex · var, assume, forget
(lowering only — a no-argument command needs no lane), choose, gcd, lcm,
factorial, is_prime, factor_integer/prime_factorization, mean (owned exact
lowering `sum(D)/len(D)`; Sage's global mean() is removed upstream).
**Deliberately deferred:** stddev (no stable Sage backend; an owned
sample-stddev expression repeats the payload three times — unreadable as
user-visible Generated Sage), the hitlist's solve `==0` normalization and plot
default range (each would flip a frozen V0.7 contract test: `solveWithoutEquals`,
`plotNoRange`), and the infix `n choose k` spelling (the shim only matches
leading command words). **Live-verified on screen** (computer-use, isolated
`CASETTE_CONFIG_DIR`): all 30 checked lanes match the integral bar's look/feel;
11 submitted results correct (sum k^2 1..10 → 385, det → −2, subs → 13, choose
→ 120, factor #1 + 2 → 2·3, …). The verification round also caught and fixed:
(1) partial range edits losing data — the IRs now re-parse through a tolerant
`parsePartialRange` (compiler lowerings stay strict) with a
`PartialEditInvarianceTests` net asserting no per-field edit sequence ever
drops typed text; (2) no keyboard path into the lane — Tab in the editor now
enters the token fields via the key-view loop when a lane is showing
(`.ignored` otherwise; Return still submits only from the editor); (3) long
equation payloads render-clipping at the fixed token ideal width — eq/eqn/
bindings/condition tokens now grow with content (capped 220pt). `make check` /
`make test` **608/608** / `make build` green.

**Previous maintenance:** Restored tape rows now read as **not live** until Replay Session recomputes them into the current Sage namespace. The tape still restores render-ready from disk, but cached rows are visually deemphasized and row-derived execution affordances (History rerun, plot regenerate, Approximate Numerically, and Actions-tab command buttons) are disabled/guarded so users do not fire commands against variables that are only present in the old persisted transcript. Replayed rows become live again and regain the normal affordances. A focused persistence test covers the stale-row guard; `make check`, `make test` (**298/298**), and `make build` are green.

**Current phase:** V1.10 complete — Sage Doctor in app. **Next:** V1.11 — Result Actions, starting by reconciling the existing V1.6 Actions-tab implementation with `plans/INITIAL.md`.

## Detailed Entries

| # | Entry | Status | Summary |
| ---: | --- | --- | --- |
| 0 | [Statistics + preload implementation](progress/000-2026-06-13-stat-preload-planning.md) | PASS | Hidden worker preload helpers plus normal/binomial/Poisson/exponential/uniform friendly commands and formula bars. |
| 1 | [V1.10: Sage Doctor in app](progress/001-2026-06-12-v1-10-sage-doctor-in-app-pass.md) | PASS | In-app Sage Doctor sheet, diagnostic checks, setup-failure recovery. |
| 2 | [V1.9: Session persistence and recovery](progress/002-2026-06-12-v1-9-session-persistence-and-recovery-pass.md) | PASS | Persistent session tape: restore, replay, crash recovery, layout memory. |
| 3 | [V1.8: Exact/numeric controls](progress/003-2026-06-12-v1-8-exact-numeric-controls-pass-live.md) | PASS | Exact/numeric display controls, precision menu, numeric replay semantics. |
| 4 | [V1.7: Plot rendering v1](progress/004-2026-06-12-v1-7-plot-rendering-v1-pass-live.md) | PASS | Inline PNG plot rendering, zoom sheet, missing-artifact honesty. |
| 5 | [V1.6: Sidebar v1](progress/005-2026-06-12-v1-6-sidebar-v1-pass-live-gate.md) | PASS | Symbols/History/Inspector/Actions sidebar workflows. |
| 6 | [V1.5: Result rendering v1](progress/006-2026-06-11-v1-5-result-rendering-v1-pass-live.md) | PASS | Math result rendering with SwiftMath and graceful fallbacks. |
| 7 | [V1.4: Input pane v1](progress/007-2026-06-11-v1-4-input-pane-v1-pass-live.md) | PASS | Input pane editing, preview, ambiguity picker, keyboard behavior. |
| 8 | [V1.3: Kernel integration](progress/008-2026-06-11-v1-3-kernel-integration-pass-live-gate.md) | PASS | Real Sage kernel connection, serial work queue, interrupt/restart. |
| 9 | [V1.2: Session model](progress/009-2026-06-11-v1-2-session-model-pass-live-gate.md) | PASS | Session row/envelope model lifted from V0.10. |
| 10 | [V1.1: App skeleton & layout](progress/010-2026-06-11-v1-1-app-skeleton-layout-pass.md) | PASS | Native app skeleton, three-region layout, build/run path. |
| 11 | [V0.10: Session tape persistence-lite](progress/011-2026-06-11-v0-10-session-tape-persistence-lite-pass.md) | PASS | Persistence proof and final V0 gate. |
| 12 | [V0.9: Sage Doctor / environment discovery](progress/012-2026-06-11-v0-9-sage-doctor-environment-discovery-pass.md) | PASS | Sage discovery/config/reporting proof. |
| 13 | [V0.8: Exact/numeric display policy](progress/013-2026-06-11-v0-8-exact-numeric-display-policy-pass.md) | PASS | Exact vs numeric worker policy proof. |
| 14 | [V0.7: Friendly input compiler (command shim → Sage)](progress/014-2026-06-11-v0-7-friendly-input-compiler-command-shim.md) | PASS | Friendly command compiler proof. |
| 15 | [V0.6: Live symbol-table introspection](progress/015-2026-06-11-v0-6-live-symbol-table-introspection-pass.md) | PASS | Live symbol-table introspection proof. |
| 16 | [V0.5: Plot & artifact pipeline](progress/016-2026-06-11-v0-5-plot-artifact-pipeline-pass-render.md) | PASS | Plot artifact pipeline proof. |
| 17 | [V0.4: LaTeX rendering in SwiftUI](progress/017-2026-06-11-v0-4-latex-rendering-in-swiftui-pass.md) | PASS | LaTeX rendering engine proof. |
| 18 | [V0.3: Result envelope & type classification](progress/018-2026-06-11-v0-3-result-envelope-type-classification-pass.md) | PASS | Result envelope classification proof. |
| 19 | [V0.2: Worker lifecycle, interrupts & restart](progress/019-2026-06-11-v0-2-worker-lifecycle-interrupts-restart-pass.md) | PASS | Worker lifecycle, interrupt, restart proof. |
| 20 | [V0.1: Sage worker protocol (the first real gate)](progress/020-2026-06-11-v0-1-sage-worker-protocol-the-first.md) | PASS | JSONL Sage worker protocol proof. |
| 21 | [Bootstrap: hello-world app + build system](progress/021-2026-06-11-bootstrap-hello-world-app-build-system.md) | done | Initial SwiftPM/macOS app build system. |
