# Code Review Policy

DEEP SAP uses two independent review layers:

1. **GitHub Actions** is authoritative for compilation, formatting, Clippy, tests, dependency audit, WebAssembly output, and release validation.
2. **CodeRabbit** reviews architecture, security boundaries, numerical assumptions, ECS design, and missing edge cases through `.coderabbit.yaml`.

CodeRabbit suggestions are advisory unless they identify a failing invariant or an issue confirmed by tests. Reviewers must reject changes that:

- allow tracking or UI systems to read hidden truth state;
- mix observations from unrelated track hypotheses;
- conceal dimensions, units, tolerances, covariance, or degenerate geometry;
- introduce browser-side provider credentials or unrestricted proxying;
- describe synthetic or public environmental data as operational target detection.

Mathematical changes require deterministic regression tests and an independent reference calculation where practical.
