---
name: verify-memoir
description: Build and drive Memoir's real optd/DataFusion SQL optimizer diagnostic example, compare query results, and retain plans and memo evidence.
---

## Launch
Use Rust 1.99.1, a native C/C++ compiler, and PowerShell 7 (`pwsh`). From the repository root run `pwsh -File tools/verification/verify.ps1 Launch`. The example executable in target/debug/examples is readiness. Cargo uses the existing lockfile, two jobs, and no debug symbols. For the local isolated toolchain, set CARGO_HOME and RUSTUP_HOME to the coursework-rust cargo and rustup directories before invocation; other machines use their own Rust installation.

## Doctor
Run `pwsh -File tools/verification/verify.ps1 Doctor`; it checks the built executable, Cargo version and current source commit. No service, credentials, external database, GPU, or cloud account is needed.

## Drive
Run `pwsh -File tools/verification/verify.ps1 Drive -Evidence evidence/memoir`. The real bridge parses three SQL queries, applies heuristic and Cascades rules, chooses costed physical operators, and executes through DataFusion. A separately seeded ordinary DataFusion context is the reference. Ordered outputs must match, including duplicates and empty results. The skewed fixture has 1000 orders, 20 customers, four regions and 80% of orders assigned to customer zero.

## Evidence
For a benchmark, build `cargo +1.99.1 build --release --locked -p optd-datafusion-bridge --example memoir --jobs 2`, then run `target/release/examples/memoir --benchmark evidence/benchmark/rows-100000 100000 5`. Repeat at 1,000 and 1,000,000 rows. `python3 tools/verification/summarize_benchmark.py evidence/benchmark` produces medians, ranges, root estimate error and artifact hashes. `.github/workflows/benchmark.yml` records runner, binary, resources and a separate diagnostic run. One-repeat runs are correctness setup only. Reuse frozen queries and sizes for any change comparison. Plans expose estimates and operator metrics; a source-level limiter remains required before a cause-dependent fix.

Run `pwsh -File tools/verification/verify.ps1 Evidence`. Each SQL action has result, verbose plan and full memo files; summary.txt is written only after every assertion passes. The driver checks hash joins, filters, costed memo winners, and multiple logical join orders. Inspect skewed_join.plan.txt to establish filter placement below joins; a mere operator name is insufficient visual proof of placement. Retain transcripts of baseline/static checks separately. Outputs reflect estimates, not measured speedups or guaranteed lower cost.

## Cleanup
Run `pwsh -File tools/verification/verify.ps1 Cleanup`, then Evidence again. Each drive exits after using isolated in-memory tables, so there is no owned server or scratch database to remove. Never delete evidence as part of cleanup. Separate evidence directories allow concurrent runs without output collisions.

## Helpers
Maintained driver: tools/verification/verify.ps1. Real product harness: optd-datafusion-bridge/examples/memoir.rs. Feature map: features/README.md. Keep the map current with affected SQL paths; use PStack maintain-verification-skill for broader drift.
