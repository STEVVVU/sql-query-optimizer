# SQL query optimizer

This project preserves the complete [CMU optd-original](https://github.com/cmu-db/optd-original) implementation and Git history. The upstream supplies the optimizer; the additions here are a skewed SQL parity executable, retained memo/plan diagnostics, a reusable verification driver and credential-free CI. This studies the [CMU 15-721 Spring 2024 project showcase](https://15721.courses.cs.cmu.edu/spring2024/project-showcase.html), without claiming enrollment, authorship of optd, or course completion.

## Run

Install Rust 1.99.1 and native C/C++ build tools. The checked-in Cargo.lock fixes the archived graph. From this directory:

```sh
cargo +1.99.1 run --locked -p optd-datafusion-bridge --example memoir -- evidence/memoir
```

The example seeds in-memory tables, parses real SQL, optimizes through the existing DataFusion bridge and Cascades engine, executes physical plans, and compares ordered results with an independent ordinary DataFusion session. It covers a filtered three-table join on skewed data, grouped aggregation and an empty result. Each query saves SQL, results, verbose plan and full costed memo. Failed parity or exploration returns a nonzero exit status.

PowerShell 7 lifecycle:

```powershell
pwsh -File tools/verification/verify.ps1 Launch
pwsh -File tools/verification/verify.ps1 Doctor
pwsh -File tools/verification/verify.ps1 Drive
pwsh -File tools/verification/verify.ps1 Evidence
pwsh -File tools/verification/verify.ps1 Cleanup
pwsh -File tools/verification/verify.ps1 Evidence
```

The driver uses two jobs and no debug symbols. Fixtures live inside the short-lived process; evidence survives cleanup. See [.agents/skills/verify-memoir](.agents/skills/verify-memoir/SKILL.md). The full upstream interactive CLI is retained: `cargo +1.99.1 run --locked --bin datafusion-optd-cli`; its larger dependency surface is outside the fixture acceptance check.

## Mechanism map

| Mechanism | Existing implementation | Acceptance evidence |
| --- | --- | --- |
| Memo groups, equivalent expressions, merging | optd-core/src/cascades/memo.rs | Core tests; memo expressions |
| Cascades tasks, scheduling, pruning | optd-core/src/cascades/tasks2.rs and optimizer.rs | Costed physical winners and join alternatives |
| Join commutation/association, filter movement | optd-datafusion-repr/src/rules | EXPLAIN alternatives and pushed filters |
| Cardinality and compute/I/O estimates | optd-datafusion-repr/src/cost | Verbose costs and row counts |
| Advanced statistics | optd-datafusion-repr-adv-cost and optd-gungnir | Preserved implementation, separate package tests |
| Logical/physical properties | optd-core/src/physical_property.rs and tests/heuristics_physical_property.rs | Existing property enforcement tests |
| SQL planning and physical execution | optd-datafusion-bridge/src | Three queries equal ordinary DataFusion |

```sh
cargo +1.99.1 test --locked -p optd-core --lib
cargo +1.99.1 test --locked -p optd-datafusion-repr --lib
cargo +1.99.1 fmt --all -- --check
cargo +1.99.1 clippy --locked -p optd-datafusion-bridge --example memoir --no-deps
```

## Provenance and limits

Fork base: `c7f727f9e862beed6be5515eb2bb223d1c8bd8c3`. [README.upstream.md](README.upstream.md) is the unchanged original overview. MIT LICENSE and Apache notices in the vendored DataFusion CLI are preserved. The pinned graph uses DataFusion 43 and Arrow 53; the upstream overview's version-32 reference is historical. No replacement optimizer or new third-party package is introduced; the example adds a direct development reference to the already-locked Tokio runtime.

The default cost model uses fallback table estimates and does not infer fixture skew. The harness proves parity, exploration and cost-based selection, not optimal runtime or calibrated cardinality. Generic physical properties have unit coverage; the SQL fixture does not assert every sort/distribution property. Upstream is research software unsuitable for production. Adaptive behavior, all SQL/types, Postgres benchmarks and cloud object stores are not exhaustively verified. The documented acceptance path needs no database service, credentials, GPU or paid compute.

The upstream bridge rewrites output field names (for example aliases become col0/col1, aggregates use generic names). Verification compares ordered typed values and expected row counts, retains separate actual/reference displays, and does not claim alias preservation. Empty results exercise zero rows; they do not prove physical schema equality when either engine emits no batches.

The new example denies Clippy warnings in its source. The no-deps Clippy command reports inherited bridge warnings; strict workspace Clippy on Rust 1.99.1 also fails on pre-existing upstream lints. The original upstream CI is preserved; the separate optimizer workflow exercises the local SQL acceptance path.
