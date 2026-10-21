# Cascades SQL search and execution

## Sub-features
Skewed three-table join, filter placement, join commutation/association, physical hash join, costed memo winners, grouping, ordering and empty outputs.

## How to get to it (user POV)
Run the memoir example via the verification driver. SQL statements are saved beside their results for inspection and replay.

## Driving it with PowerShell
Run Launch, Doctor, Drive, Evidence, Cleanup and Evidence as documented in the parent skill. All three query results must equal a separately seeded DataFusion reference. Inspect the saved memo alternatives and EXPLAIN chosen operators; verify pushed filters appear beneath the relevant joins in skewed_join.plan.txt.

## Gotchas
The default cost model does not learn the skew automatically. This fixture proves execution parity and exploration, not estimator accuracy or optimal runtime. Hash map diagnostic ordering may vary, so do not snapshot full memo text. The archived upstream has unsupported SQL/types and production limitations. The full interactive CLI, adaptive workloads and external benchmark tools are separate upstream surfaces and are not covered here.
