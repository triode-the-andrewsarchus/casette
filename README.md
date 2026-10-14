## Casette: a simple CAS calculator and SageMath frontend.

This project is a fork of [casette](https://github.com/tqbf/casette)

In short,

* It's a calculator.

* It keeps a running tape of expressions.

* You can click an expression to expand, simplify, transpose, or take a derivative of a prior value.

* You can refer to prior values by their entry number on the tape, like `diff(#13, x)`. 

* It renders LaTeX math. 

* In addition to accepting straight Sage-flavored Python, there's also a poorly-documented and somewhat extensive shortcut syntax, and a graphical formula expander that I shoplifted from Numbers.app.

* *It accepts Matlab matrix definition notation*.

* It handles units.

Importantly: this fork is written in **Rust** using **egui** (`eframe`) for high-performance native Linux desktop execution, with a Tokio-based process manager connecting to a persistent SageMath background worker. Knock yourself out!

### Building & Running (Linux)

Requires Rust (1.80+) and SageMath (Sage 10+ / Python 3 with `sage.all`).

```bash
make setup    # Detect distro and install dependencies (pacman on Arch, conda on Ubuntu)
make run      # Build and launch the Casette GUI
make test     # Run all unit, lowering, and kernel integration tests
make check    # Run compiler checks and clippy
make release  # Build optimized release binary
make install  # Install to ~/.local/bin and add desktop launcher
```

