# Problems & Hard-Won Lessons

This is the index for the active, applicable problem notes in [`problems/`](problems/).
Each entry preserves the verified operational lesson for the SageMath worker, process management, friendly compiler, and session persistence. Entries are ordered newest first.

| # | Problem | Intersects |
| ---: | --- | --- |
| 2 | [Restart re-init must CHAIN on the serial kernel queue — only the kill/boot itself may run un-chained, or the next submission races the fresh worker's preludes](problems/002-restart-re-init-must-chain-on-the-serial-kernel.md) | Restart ordering, queued worker re-init, post-restart state races. |
| 9 | [The V0.7 friendly compiler FLATTENS NEWLINES — route multiline input around it, or raw Sage gets corrupted](problems/009-the-v0-7-friendly-compiler-flattens-newlines-route-multiline.md) | Multiline input routing, friendly compiler boundaries, raw Sage preservation. |
| 10 | [`posix_spawn` children INHERIT the parent's signal dispositions — spawn the worker with `POSIX_SPAWN_SETSIGDEF` + `SETSIGMASK` or interrupts silently die](problems/010-posix-spawn-children-inherit-the-parent-s-signal-dispositions.md) | POSIX signal inheritance, SIGINT delivery, worker signal masks. |
| 11 | [Session persistence: a missing artifact is the NORMAL case, paths aren't identity, and peek the schema before decoding](problems/011-session-persistence-a-missing-artifact-is-the-normal-case.md) | Session artifact liveness, path identity, schema peek-before-decode. |
| 13 | [Exactness: the Symbolic Ring is uniformly `is_exact()==False` — decide per-kind, and tree-walk symbolic](problems/013-exactness-the-symbolic-ring-is-uniformly-is-exact-false.md) | Exactness policy, Symbolic Ring behavior, tree-walked exactness. |
| 14 | [Numeric precision: clamp the request to the value's own `prec()` or `.n(bits)` raises](problems/014-numeric-precision-clamp-the-request-to-the-value-s.md) | Numeric precision bounds, value precision, safe numeric requests. |
| 15 | [Friendly compiler: the bypass rule is a SPACE boundary, and a definite integral is `symbolic`](problems/015-friendly-compiler-the-bypass-rule-is-a-space-boundary.md) | Friendly compiler parsing edges, bypass boundaries, definite integral kind. |
| 16 | [Symbol table: diff the baseline by OBJECT IDENTITY, not name — Sage exports `n`/`N`/`i`](problems/016-symbol-table-diff-the-baseline-by-object-identity-not.md) | Sage symbol snapshots, baseline diffing, object identity vs names. |
| 20 | [Classifying Sage results: type module lies in three specific ways](problems/020-classifying-sage-results-type-module-lies-in-three-specific.md) | Sage type classification, module-name traps, result-kind mapping. |
| 21 | [Numeric approximation: respect the value's native precision; `digits=` can raise](problems/021-numeric-approximation-respect-the-value-s-native-precision-digits.md) | Approximation API behavior, native precision, digits pitfalls. |
| 22 | [Interrupting Sage: cysignals owns SIGINT and makes C code abortable — don't clobber it](problems/022-interrupting-sage-cysignals-owns-sigint-and-makes-c-code.md) | Interrupt semantics, cysignals, abortable C/Sage computations. |
| 23 | [Restart race: give each worker generation its own reader queue + EOF event](problems/023-restart-race-give-each-worker-generation-its-own-reader.md) | Worker generations, reader queues, restart/EOF race isolation. |
| 24 | [`sage -python` is a bash wrapper — kill the process *group*, not the PID](problems/024-sage-python-is-a-bash-wrapper-kill-the-process.md) | sage-python wrapper processes, process-group cleanup, kill targeting. |
| 25 | [Protocol stdout must use a *private dup'd fd*, and capture must redirect fd 1/2 (not just `sys.stdout`)](problems/025-protocol-stdout-must-use-a-private-dup-d-fd.md) | Worker protocol stdout/stderr capture, private fds, fd redirection. |
| 26 | [Sage value-echo: parse the *preparsed* code with `ast`, and suppress `None`](problems/026-sage-value-echo-parse-the-preparsed-code-with-ast.md) | Sage value echo, preparser AST parsing, suppressing None. |
