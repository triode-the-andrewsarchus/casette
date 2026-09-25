# Casette Units System & Natural Unit Parser (V2.0)

## Overview

Casette provides natural, dimensional physical unit handling backed by SageMath's
`sage.symbolic.units` engine. Instead of requiring verbose, cumbersome Sage paths
like `32 * units.length.mile` and manual conversions, Casette allows users to write:

```text
solve 32*mi*x^2 = 52*mi^2
32 mi + 5 km to miles
100 km / 2 hr in mph
9.8 m/s^2 * (5 s)^2
```

---

## 1. Syntax & Grammar

### 1.1 Unit Quantities
A quantity can be written with explicit multiplication or natural juxtaposition:
- **Explicit multiplication:** `32 * mi`, `52 * mi^2`, `(100 * km) / (2 * hr)`
- **Natural juxtaposition:** `32 mi`, `52 mi^2`, `9.8 m/s^2`
- **Unit powers:** `mi^2` binds tightly to the unit (`52 mi^2` $\to$ `52 * (mi^2)`).
- **Compound units:** `m/s^2`, `km/hr`, `kg*m/s^2`, `N*m`, `kW*hr`.

### 1.2 Unit Conversion Clauses
Any expression can be followed by a conversion clause:
- `<expr> to <unit>`
- `<expr> in <unit>`

Examples:
- `32 mi + 5 km to km` $\to$ `__casette_convert(32*mi + 5*km, km)`
- `100 km / 2 hr in mph` $\to$ `__casette_convert((100*km) / (2*hr), mph)`

### 1.3 Implicit Unit Simplification & Solving
When no explicit `to` / `in` target is provided:
1. **Dimensionless cancellation:** If an expression simplifies to a dimensionless scalar
   (e.g. `(50*km) / (32*mile)`), units cancel automatically to a pure number.
2. **Compatible unit summation:** Adding compatible units (e.g. `32*mi + 5*km`) unifies
   into the dominant/first unit.
3. **Equation solving:** In `solve 32*mi*x^2 = 52*mi^2`:
   - `mi` is recognized as a known unit of length, not a variable.
   - `x` is recognized as the only algebraic unknown.
   - Sage solves for `x` automatically without ambiguity.

---

## 2. Standard Unit Vocabulary

The unit registry covers standard SI and Imperial units:
- **Length:** `m`, `meter`, `meters`, `km`, `cm`, `mm`, `mi`, `mile`, `miles`, `ft`, `feet`, `foot`, `in`, `inch`, `inches`, `yd`, `yard`
- **Time:** `s`, `sec`, `second`, `seconds`, `ms`, `min`, `minute`, `minutes`, `hr`, `hour`, `hours`, `day`, `days`, `yr`, `year`
- **Mass:** `kg`, `kilogram`, `g`, `gram`, `mg`, `lb`, `lbs`, `pound`, `pounds`, `oz`, `ounce`
- **Force & Energy:** `N`, `newton`, `J`, `joule`, `W`, `watt`, `kW`, `cal`, `kcal`, `eV`
- **Pressure:** `Pa`, `pascal`, `bar`, `atm`, `psi`
- **Speed (compound aliases):** `mph`, `kph`
- **Angle:** `deg`, `degree`, `degrees`, `rad`, `radian`, `radians`

---

## 3. Architecture & Implementation

### 3.1 Worker Runtime Engine (`worker.py`)
Preloads a lightweight unit runtime in the Sage worker:
- Global shorthand aliases (`mi = units.length.mile`, `km = units.length.kilometer`, etc.).
- `__casette_convert(expr, target=None)` implementation handling:
  - Explicit conversion via `sage.symbolic.units.convert(expr, target)`.
  - Implicit dimensionless reduction via substitution of units into base units.
  - Automatic unification of sums of compatible units.

### 3.2 Compiler Lexer & Parser (`logos` + `nom`)
In `crates/friendly-compiler`:
- `units.rs`: Unit registry, aliases, dimensional classification, and variable exclusion.
- `unit_parser.rs`: `logos` tokenization and `nom` combinators for quantity parsing,
  exponent binding, compound unit fractions, and `to`/`in` conversion clauses.
- `variables.rs`: `free_variables` ignores registered unit symbols so algebraic equations
  correctly isolate unknown variables.
