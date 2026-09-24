## Casette: a simple CAS calculator and SageMath frontend.

![Casette Screenshot](CASETTE.png)

I spent a year doing Math Academy, starting with a grim confrontation with fractions and ending with multivariable calculus. Starting with MVC, after earning the privilege by grinding months of trig identity integrals, I adopted [SageMath](https://www.sagemath.org/) as my daily companion. 

Sage is fantastic, but it's 2026, and CLI/TUI interfaces are obsolete. The Sage REPL is not  especially discoverable (I was today less 1 month old when I discovered '_'). I would rather eat a bug than write Python directly into a Jupyter notebook. 

So this:

* It's a calculator.

* It keeps a running tape of expressions.

* You can click an expression to expand, simplify, transpose, or take a derivative of a prior value.

* You can refer to prior values by their entry number on the tape, like `diff(#13, x)`. 

* It renders LaTeX math. 

* In addition to accepting straight Sage-flavored Python, there's also a poorly-documented and somewhat extensive shortcut syntax, and a graphical formula expander that I shoplifted from Numbers.app.

* *It accepts Matlab matrix definition notation*.

It will probably get better. Importantly: this repo is written in **Rust** using **egui** (`eframe`) for high-performance native Linux desktop execution, with a Tokio-based process manager connecting to a persistent SageMath background worker. Knock yourself out!

### Building & Running (Linux)

Requires Rust (1.80+) and SageMath (Sage 10+ / Python 3 with `sage.all`).

```bash
make run      # Build and launch the Casette GUI
make test     # Run all unit, lowering, and kernel integration tests
make check    # Run compiler checks and clippy
make release  # Build optimized release binary
make install  # Install to ~/.local/bin and add desktop launcher
```

