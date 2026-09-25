# Woxi Math Engine Migration Plan

## 1. Overview & Goal
Replace the external SageMath backend (`worker.py` spawned via `SageKernel`) with an in-process, pure Rust Wolfram Language interpreter using **Woxi** (`https://github.com/ad-si/Woxi`).

This transition:
- Eliminates the external runtime dependency on Python 3 and SageMath (which previously required gigabytes of installation and slow subprocess cold-starts).
- Provides instant, sub-millisecond kernel startup directly within the Rust application.
- Retains full support for symbolic math, calculus (differentiation, integration), algebra (solve, factor, expand), linear algebra, matrices, tape references (`#ROW`), and natural physical unit arithmetic.

## 2. Core Prerequisite: Woxi Source Dependency & UnitSimplify
Woxi could not previously be dropped in as-is because it lacked the `UnitSimplify` operation ([Woxi Conformance Gaps](https://woxi.ad-si.com/docs/comparison/mathematica/conformance_gaps/#unitsimplify-is-unimplemented)).

### Resolution in Vendored Woxi (`vendor/woxi`):
1. **Vendoring:** Cloned Woxi into `vendor/woxi` and linked via Cargo path dependency (`woxi = { path = "../../vendor/woxi", default-features = false }`) with workspace exclusion in the root `Cargo.toml`.
2. **`UnitSimplify` Implementation:**
   - Implemented in `vendor/woxi/src/functions/quantity_ast.rs` with dispatch in `vendor/woxi/src/evaluator/dispatch/quantity_functions.rs`.
   - Dimensionless scalar reduction: when dimensions cancel completely (e.g. `UnitSimplify[50*km / (32*mi)]`), automatically evaluates to the pure rational or decimal scalar (`390625/402336`).
   - Derived SI unit simplification: groups base SI dimensions (`kg`, `m`, `s`, `A`) into standard named derived units (`Newtons`, `Joules`, `Watts`, `Pascals`, `Volts`, `Coulombs`, `Farads`, `Ohms`, `Siemens`, `Webers`, `Teslas`, `Henries`).
   - Compound unit simplification and recursive expression traversal (simplifying units inside equations, lists, rules, and arithmetic trees).
   - Quantity unwrapping in unit arguments so `Quantity[1, "Kilometers"]` can be used interchangeably with `"Kilometers"`.
   - Verified with unit tests (`vendor/woxi/tests/interpreter_tests/quantity.rs: unit_simplify_tests`).
3. **Dimensional Equation Solving (`Solve` with Units):**
   - Implemented in `vendor/woxi/src/functions/polynomial_ast/solve.rs` (`try_solve_quantity_equation`).
   - Automatically determines the target unit for the unknown variable from the equation (e.g. `ft` in `3 mi + x ft = 5 km`, `s` in `3 hr + x sec = 12 days`).
   - Converts all dimensional terms to the target unit via `unit_convert_ast`, strips `Quantity` wrappers to yield an algebraic scalar polynomial equation, solves the equation via `solve_ast`, and re-wraps solutions into `Quantity[solution, target_unit]`.
   - Visual output form renders unit rules cleanly as `{{x -> 214960/381 ft}}`.
   - LaTeX output formats `Quantity` values with `\text{ <unit>}` (e.g. `\frac{214960}{381}\text{ ft}`).

## 3. Kernel Architecture (`casette-kernel`)

### Thread Isolation & Session State
- Woxi utilizes thread-local storage (`ENV`, `FUNC_DEFS`, `MEMO_VALUES`, etc.) for its evaluation context.
- `casette-kernel` runs Woxi inside a dedicated background worker OS thread (`std::thread::spawn`).
- Communication with Tokio async callers uses `tokio::sync::mpsc` for request dispatch and `oneshot::Sender` for replies.
- Session restarts call `woxi::clear_state()` and re-run initialization preludes, ensuring a deterministic, leak-free, clean state without needing OS process killing.

### Boot Prelude & Symbol Setup
On kernel boot and after any restart:
1. Clear session state: `woxi::clear_state()`.
2. Define common physical units as `Quantity` objects:
   - Length: `km`, `mi`, `m`, `cm`, `mm`, `ft`, `inch`, `yard`.
   - Time: `s`, `sec`, `minute`, `hr`, `day`.
   - Mass: `kg`, `g`, `lb`, `oz`.
   - Force / Energy / Power: `newton`, `joule`, `watt`, `pascal`.
   - Electromagnetism: `volt`, `amp`, `coulomb`, `ohm`.
   - Velocity: `mph`, `kph`.
3. Preload Casette conversion shims:
   ```wolfram
   __casette_convert[expr_, target_] := UnitConvert[expr, target];
   __casette_convert[expr_] := UnitSimplify[expr];
   ```
4. Preload mathematical function aliases for smooth compatibility:
   ```wolfram
   factor[x_] := Factor[x];
   expand[x_] := Expand[x];
   simplify[x_] := Simplify[x];
   diff[f_, x_] := D[f, x];
   integral[f_, x_] := Integrate[f, x];
   integrate[f_, x_] := Integrate[f, x];
   solve[eq_, x_] := Solve[eq, x];
   det[m_] := Det[m];
   ```

### Request & Result Pipeline
1. **Input Normalization / Syntax Translation:**
   - Translates Python/Sage-style function call syntax into Wolfram Language bracket syntax when appropriate (`factor(...)` $\to$ `factor[...]`, `solve(..., ...)` $\to$ `solve[...]`, `matrix(...)` $\to$ `matrix[...]`).
   - Translates matrix lists `[[a, b], [c, d]]` to Wolfram nested lists `{{a, b}, {c, d}}`.
   - Expands `#ROW` references to `__casette_tape_refs[ROW]`.
2. **Evaluation:**
   - Evaluates via `woxi::interpret_with_stdout(&code)`.
   - If numeric approximation requested, evaluates `N[expr, digits]`.
3. **Response Envelope (`WorkerResponse`):**
   - `plain`: string representation from `InterpretResult.result`.
   - `latex`: generated via `woxi::functions::string_ast::expr_to_tex(&expr)`.
   - `kind`: classified from `syntax::Expr` (`"integer"`, `"real"`, `"rational"`, `"complex"`, `"quantity"`, `"list"`, `"symbolic"`, `"boolean"`, `"plot"`, etc.).
   - `stdout`: captured from `InterpretResult.stdout`.
   - `artifacts`: if `graphics` / `output_svg` produced, written as SVG image artifact in session temporary storage and referenced via `ArtifactPayload`.
   - `approx`: evaluated if result has numerical value.

## 4. UI & Compiler Integration
- Update `casette-gui/src/main.rs` to launch the Woxi-backed kernel.
- Tape reference storage:
  - When a row succeeds, assign `__casette_tape_refs[row_num] = (<expr>);` as a Woxi DownValue.
  - On clear tape: `Clear[__casette_tape_refs];`.
- Symbols Tab:
  - Query user definitions from Woxi session and present in the Symbols sidebar table.
