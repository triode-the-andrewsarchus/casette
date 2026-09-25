# Problems & Hard-Won Lessons

This is the index for the active, applicable problem notes in [`problems/`](problems/).
Each entry preserves the verified operational lesson for the SageMath worker, process management, friendly compiler, and session persistence. Entries are ordered newest first.

| # | Problem | Intersects |
| ---: | --- | --- |
| 21 | [Numeric approximation: respect the value's native precision; `digits=` can raise](problems/021-numeric-approximation-respect-the-value-s-native-precision-digits.md) | Approximation API behavior, native precision, digits pitfalls. |
| 22 | [Interrupting Sage: cysignals owns SIGINT and makes C code abortable — don't clobber it](problems/022-interrupting-sage-cysignals-owns-sigint-and-makes-c-code.md) | Interrupt semantics, cysignals, abortable C/Sage computations. |
| 23 | [Restart race: give each worker generation its own reader queue + EOF event](problems/023-restart-race-give-each-worker-generation-its-own-reader.md) | Worker generations, reader queues, restart/EOF race isolation. |
| 24 | [`sage -python` is a bash wrapper — kill the process *group*, not the PID](problems/024-sage-python-is-a-bash-wrapper-kill-the-process.md) | sage-python wrapper processes, process-group cleanup, kill targeting. |
| 25 | [Protocol stdout must use a *private dup'd fd*, and capture must redirect fd 1/2 (not just `sys.stdout`)](problems/025-protocol-stdout-must-use-a-private-dup-d-fd.md) | Worker protocol stdout/stderr capture, private fds, fd redirection. |
| 26 | [Sage value-echo: parse the *preparsed* code with `ast`, and suppress `None`](problems/026-sage-value-echo-parse-the-preparsed-code-with-ast.md) | Sage value echo, preparser AST parsing, suppressing None. |
