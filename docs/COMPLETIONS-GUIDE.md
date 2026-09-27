# Friendly Compiler & Completions Guide

This guide is the complete command reference for Casette's friendly input mini-language.

Casette includes a pure-Rust compiler (`crates/friendly-compiler`) that translates intuitive mathematical shorthand, friendly command words, Matlab-style matrix notation, and physical dimensional expressions into executable SageMath code in real time.

---

## Conventions & Rules

- **Friendly in, Sage out:** The editor accepts friendly human-readable commands. The `↪ Generated Sage` preview line beneath the editor displays the exact Sage code that will be evaluated upon pressing **Enter**.
- **Template Chips:** The toolbar above the editor provides one-click template insertion chips (`factor`, `expand`, `diff`, `integral`, `solve`, `matrix`, `plot`, `taylor`, `sum`).
- **`#ROW` Tape References:** Expressions can reference prior row results using `#1`, `#2`, etc. (`factor #14 + 2`, `plot #1, x=-10..10`). References remain verbatim in input and history, expanding to `__casette_tape_refs[ROW]` at the compile boundary.
- **Call Syntax Bypasses:** A command word followed by a space or arguments triggers friendly compilation. Standard function call syntax with parentheses (e.g. `det(A)`, `factorial(5)`, `diff(x^2, x)`) bypasses the friendly compiler and is passed directly to SageMath untouched.
- **Multi-line Bypass:** Multi-line inputs (containing `\n`) bypass single-line compiler normalization, preserving multi-line Python/Sage code structures.
- **Ranges:** Ranges are written as `var=lo..hi` (e.g. `x=0..1` or `x=-pi..pi`).

---

## Expression Transforms

| Trigger | Aliases | Syntax / Example | Lowers to |
| --- | --- | --- | --- |
| `expand` | — | `expand (x+1)^5` | `expand((x+1)^5)` |
| `factor` | — | `factor x^4 - 1` | `factor(x^4 - 1)` |
| `simplify` | — | `simplify sin(x)^2 + cos(x)^2` | `(…).simplify_full()` |
| `latex` | — | `latex x^2/2` | `latex(x^2/2)` |

---

## Equation Solving

```text
solve x^2 + 5*x + 6 = 0            →   solve(x^2 + 5*x + 6 == 0, x)
solve x^2 - 4 = 0 for x            →   solve(x^2 - 4 == 0, x)
solve 3*x + 2*y = 5 for x          →   solve(3*x + 2*y == 5, x)
```

- A single `=` is automatically translated to relational equality `==`.
- Without an explicit `for <var>` clause, the free variable is inferred automatically. If multiple unknown variables exist, all are solved or an ambiguity candidate set is reported.
- An explicit equation (with `=`) is required.

---

## Calculus

```text
integral x^2                       →   integrate(x^2, x)
integral x^2, x=0..1               →   integrate(x^2, (x, 0, 1))
integral x*y, wrt y                →   integrate(x*y, y)
double integral x*y, x=0..1, y=0..x
                                   →   integrate(integrate(x*y, (y, 0, x)), (x, 0, 1))

derivative sin(x^2)                →   derivative(sin(x^2), x)
  (alias: diff)
derivative sin(x), 2               →   derivative(sin(x), x, 2)
derivative x*y, 2 wrt y            →   derivative(x*y, y, 2)

limit sin(x)/x, x->0               →   limit(sin(x)/x, x=0)
limit 1/x, x->0, right             →   limit(1/x, x=0, dir='+')
limit 1/x, x->0, left              →   limit(1/x, x=0, dir='-')

taylor sin(x), x=0, order=7        →   taylor(sin(x), x, 0, 7)

sum k^2, k=1..n                    →   sum(k^2, k, 1, n)
product 1 + 1/k, k=1..n            →   product(1 + 1/k, k, 1, n)
```

---

## Plotting

```text
plot sin(x), x=-pi..pi             →   plot(sin(x), (x, -pi, pi))
plot #1, x=-10..10                 →   plot(__casette_tape_refs[1], (x, -10, 10))

parametric_plot (cos(t), sin(t)), t=0..2*pi
  (alias: parametric plot)         →   parametric_plot((cos(t), sin(t)), (t, 0, 2*pi))

implicit_plot x^2 + y^2 = 1, x=-2..2, y=-2..2
  (alias: implicit plot)           →   implicit_plot(x^2 + y^2 == 1, (x, -2, 2), (y, -2, 2))
```

- In `plot`, comma-free range syntax is also accepted (e.g. `plot sin(x) x=-pi..pi` or `plot sin(x) -pi..pi`).
- Plots render interactively using `egui_plot` with panning, mouse-wheel zooming, reset buttons, and magnetic snapping for local extrema (peaks/valleys) and zero-crossings (roots).

---

## Linear Algebra

Matrix payloads accept standard Sage nested lists (`[[1,2],[3,4]]`) and Matlab-style bracket notation (`[1 2; 3 4]`). Method commands can be applied to matrix literals, variable names, or tape references.

| Trigger | Aliases | Syntax / Literal | Variable / Reference | Lowers to |
| --- | --- | --- | --- | --- |
| `matrix` | — | `matrix [1 2; 3 4]` | — | `matrix([[1, 2], [3, 4]])` |
| `vector` | — | `vector [1, 2, 3]` | — | `vector([1, 2, 3])` |
| `det` | `determinant` | `det [1 2; 3 4]` | `det A`, `det #1` | `matrix(…).det()`, `(A).det()` |
| `inverse` | `inv` | `inverse [1 2; 3 4]` | `inverse M` | `(M).inverse()` |
| `transpose` | — | `transpose [1 2; 3 4]` | `transpose M` | `(M).transpose()` |
| `rank` | — | `rank [1 2; 3 4]` | `rank M` | `(M).rank()` |
| `rref` | — | `rref [1 2; 3 4]` | `rref M` | `(M).rref()` |
| `eigenvalues` | `eigenvalue` | `eigenvalues M` | `eigenvalues #1` | `(M).eigenvalues()` |
| `eigenvectors`| `eigenvector` | `eigenvectors M` | `eigenvectors #1` | `(M).eigenvectors_right()` |

- Rendered matrices display interactive cell-padded grids with zero de-emphasis and live matrix property badges in the Inspector tab.

---

## Physical Units & Dimensional Analysis

Casette provides full support for natural physical unit syntax and conversions backed by SageMath's physical unit library:

```text
50 km / (32 mi)                    →   __casette_convert((50*km) / (32*mi))
100 km / 2 hr in mph               →   __casette_convert((100*km) / (2*hr), mph)
32 mi to feet                      →   __casette_convert(32*mi, foot)
120 V * 15 A                       →   1800*watt  (automatic derived SI reduction)
solve 100 ohms * (x * A) = 10 V    →   [x == 1/10*A]
```

- **Juxtaposition Desugaring:** Quantities with units like `32 mi`, `9.8 m/s^2`, `x sec`, and `(x + 1) hr` automatically desugar into parenthesized multiplication `(32*mi)`, `(x*sec)`, etc.
- **Conversion Clauses:** `... to <unit>` and `... in <unit>` trigger exact dimensional conversions.
- **Automatic SI Reductions:** Compatible compound electrical and mechanical units simplify automatically (e.g. `V/A` $\to$ $\Omega$, `V*A` $\to$ $\text{W}$, `A*ohm` $\to$ $\text{V}$).

---

## Vector Calculus

```text
gradient x^2 + y^2                 →   (x^2 + y^2).gradient()
  (alias: grad)
gradient x^2*y + z, [x,y,z]        →   (x^2*y + z).gradient([x, y, z])

hessian x^2 + x*y + y^2            →   (x^2 + x*y + y^2).hessian()

jacobian [x^2+y, sin(x*y)], [x,y]  →   jacobian([x^2+y, sin(x*y)], [x, y])
```

---

## Substitution & Numeric Evaluation

```text
subs x^2 + y, x=3, y=4             →   (x^2 + y).subs(x=3, y=4)
  (alias: substitute)

numeric pi                         →   N(pi)
  (aliases: approx, decimal)
numeric sqrt(2), 50                →   N(sqrt(2), digits=50)
```

---

## Symbols, Variables & Assumptions

```text
var a b c                          →   var('a b c')
var a, b, c                        →   var('a b c')

assume x > 0                       →   assume(x > 0)
assume x real                      →   assume(x, 'real')
assume n is integer                →   assume(n, 'integer')
assume x positive                  →   assume(x > 0)
assume x negative                  →   assume(x < 0)

forget                             →   forget()
forget all                         →   forget()
```

- Variable assignments (`a = 5`), function declarations (`f(x) = x^2`), and symbolic variables (`var('t')`) persist across worker resets and tape clears.
- Active variables are listed in the **Symbols** tab in the sidebar with filter search, built-in variable toggle, and a delete button (`🗑`).

---

## Combinatorics & Number Theory

```text
choose 10, 3                       →   binomial(10, 3)
factorial 5                        →   factorial(5)
gcd 12, 18                         →   gcd(12, 18)
gcd [12, 18, 24]                   →   gcd([12, 18, 24])
gcd 12, 18, 24                     →   gcd([12, 18, 24])
lcm 4, 6                           →   lcm(4, 6)
is_prime 104729                    →   is_prime(104729)
factor_integer 3600                →   factor(3600)
  (alias: prime_factorization)
```

---

## Statistics & Distributions

```text
mean [1, 2, 3]                     →   sum([1, 2, 3])/len([1, 2, 3])

normal_pdf 0                       →   normal_pdf(0)
normal_cdf 1.96                    →   normal_cdf(1.96)
normal_between -1, 1               →   normal_between(-1, 1)
normal_inv .975                    →   normal_inv(.975)

binomial_pmf 3, n=10, p=.5         →   binomial_pmf(3, n=10, p=.5)
binomial_cdf 3, n=10, p=.5         →   binomial_cdf(3, n=10, p=.5)
binomial_between 3, 7, n=10, p=.5  →   binomial_between(3, 7, n=10, p=.5)
binomial_at_most 3, n=10, p=.5     →   binomial_at_most(3, n=10, p=.5)
binomial_at_least 8, n=10, p=.5    →   binomial_at_least(8, n=10, p=.5)

poisson_pmf 2, lambda=3            →   poisson_pmf(2, lambda_=3)
poisson_cdf 2, lambda=3            →   poisson_cdf(2, lambda_=3)
poisson_between 1, 4, lambda=3     →   poisson_between(1, 4, lambda_=3)

exponential_pdf 1, rate=2          →   exponential_pdf(1, rate=2)
exponential_cdf 1, rate=2          →   exponential_cdf(1, rate=2)

uniform_pdf .5, min=0, max=1       →   uniform_pdf(.5, low=0, high=1)
uniform_cdf .5, min=0, max=1       →   uniform_cdf(.5, low=0, high=1)
```

- Distribution helper functions are preloaded in the worker namespace so raw SageMath code and friendly input share exact, fast computation without cluttering the user's initial symbol table.

---

## Architecture & Adding New Commands

To add or extend friendly commands in the Rust codebase:
1. **Parser & Lexer (`crates/friendly-compiler/src/`):**
   - Add token rules in `scanner.rs` or `unit_parser.rs`.
   - Implement command parsing in `compiler.rs` returning a structured IR.
2. **Lowering (`crates/friendly-compiler/src/compiler.rs`):**
   - Translate the IR to an unambiguous SageMath expression string.
   - Detect free variables using `variables.rs` to populate `required_variables` (variables needing automatic `var(...)` definitions).
3. **Tests (`crates/friendly-compiler/tests/`):**
   - Add unit tests in `friendly_compiler_tests.rs` or `batch_lowering_tests.rs`.

