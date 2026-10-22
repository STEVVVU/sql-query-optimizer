# Verified SQL paths

- [Cascades search and execution](sql.md): skewed three-table join, grouped aggregate, empty result, memo diagnostics.
- `memoir --benchmark OUTPUT ORDERS REPEATS` drives the same bridge against ordinary DataFusion at 1,000, 100,000 and 1,000,000 orders. Five alternating pairs cover skewed and selective joins, grouped aggregate, scan and empty results. Each side uses four target partitions and batch 8192. Typed values must match. Raw planning/execution/total samples, row counts, root estimates, executed plans with metrics and verbose optd diagnostics remain in OUTPUT. Root Q-error is separate from optd's internal cost estimates. Fixture setup is outside timing; there is no file-I/O claim.
