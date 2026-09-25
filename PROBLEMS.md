# Problems & Hard-Won Lessons

This is the index for the active, applicable problem notes in [`problems/`](problems/).
Each entry preserves the verified operational lesson for the SageMath worker, process management, friendly compiler, and session persistence. Entries are ordered newest first.

| # | Problem | Intersects |
| ---: | --- | --- |
| 24 | [`sage -python` is a bash wrapper — kill the process *group*, not the PID](problems/024-sage-python-is-a-bash-wrapper-kill-the-process.md) | sage-python wrapper processes, process-group cleanup, kill targeting. |
| 25 | [Protocol stdout must use a *private dup'd fd*, and capture must redirect fd 1/2 (not just `sys.stdout`)](problems/025-protocol-stdout-must-use-a-private-dup-d-fd.md) | Worker protocol stdout/stderr capture, private fds, fd redirection. |
| 26 | [Sage value-echo: parse the *preparsed* code with `ast`, and suppress `None`](problems/026-sage-value-echo-parse-the-preparsed-code-with-ast.md) | Sage value echo, preparser AST parsing, suppressing None. |
