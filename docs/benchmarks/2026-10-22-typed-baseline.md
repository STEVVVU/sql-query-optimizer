# Optimizer measurements with typed result parity

[Hosted run 39730291910](https://github.com/STEVVVU/sql-query-optimizer/actions/runs/39730291910) measured source `69466cac99cce2051a0a6895528af329de3f44ce`. Artifact `optimizer-benchmark-39730291910` contains all SQL, plans with operator metrics, verbose optd estimates, raw samples, results, environment and resources. Its summary SHA-256 is `b91cf4f20584a931e59d18fc5f5100d2d5df10245b438dd84d33af405c818d25`.

The observed runner was Ubuntu with Linux 6.17.0-1022-azure, two assigned AMD EPYC 9V74 physical cores and four logical CPUs, Rust 1.99.0 and Cargo 1.99.0, using release optimization. The repository's stable toolchain override selected 1.99.0 despite the workflow installing 1.99.1. The recorded compiler is the reproducibility target for these values. The initial run used a different EPYC model and is retained separately; differences between runners are not optimizer improvements.

The deterministic in-memory fixture has 20 customers, four regions and the order counts below. Eighty percent of orders belong to customer zero. Both sides use four target partitions and batch size 8192. The optd side uses its standalone logical/Cascades pipeline; the baseline uses DataFusion's default optimizer. Five alternating pairs per query follow an excluded warmup. All 180 actions completed, including 150 measured samples and 30 warmups. Ordered row values and physical output types matched for every query, including empty outputs. Fixture construction, context creation and result conversion are outside the timers; parsing, planning and complete query execution are included.

Values are combined planning/execution milliseconds, median [minimum, maximum] of five measurements. All query/data-size pairs are shown.

| Orders | Query | optd | DataFusion |
|---:|---|---:|---:|
| 1,000 | skewed_join | 6.514 [6.418, 6.704] | 2.032 [2.025, 2.064] |
| 1,000 | aggregate | 5.767 [5.743, 5.881] | 2.201 [2.183, 2.265] |
| 1,000 | selective_join | 1.383 [1.355, 1.415] | 1.265 [1.209, 1.307] |
| 1,000 | scan | 0.494 [0.486, 0.508] | 0.549 [0.534, 0.565] |
| 1,000 | empty | 1.335 [1.328, 1.358] | 1.147 [1.138, 1.162] |
| 100,000 | skewed_join | 7.217 [7.033, 7.255] | 3.192 [3.082, 4.377] |
| 100,000 | aggregate | 7.632 [7.486, 7.710] | 7.983 [7.585, 9.758] |
| 100,000 | selective_join | 1.639 [1.612, 1.652] | 1.454 [1.376, 1.496] |
| 100,000 | scan | 0.409 [0.399, 0.455] | 0.719 [0.702, 0.960] |
| 100,000 | empty | 1.300 [1.273, 1.326] | 1.230 [1.194, 1.272] |
| 1,000,000 | skewed_join | 13.683 [13.565, 13.931] | 13.397 [11.955, 15.752] |
| 1,000,000 | aggregate | 25.350 [25.262, 25.549] | 50.680 [44.784, 55.704] |
| 1,000,000 | selective_join | 5.023 [4.948, 5.112] | 3.120 [2.441, 3.223] |
| 1,000,000 | scan | 0.956 [0.875, 0.958] | 1.318 [1.260, 1.380] |
| 1,000,000 | empty | 1.965 [1.919, 1.985] | 2.340 [1.846, 2.363] |

The result is mixed. At one million orders, aggregate planning medians were 6.010 ms optd and 2.267 ms DataFusion; execution medians were 19.342 and 48.397 ms. optd chose a join that builds ten filtered dimension rows before probing the orders. DataFusion's retained plan builds the million-row intermediate for its later regions join and repartitions it. This explains the observed execution gap for that query, while optd pays more planning overhead. Component medians need not sum to the combined median.

The selective join has the opposite execution result. optd probes all one million orders against one customer; DataFusion propagates `customer_id=1` and probes 10,000 filtered orders. The separate 100k-row diagnostic run reproduces the same 100,000-versus-1,000 probe counts. This is a concrete missing filter-propagation path, not evidence that optd is uniformly slower or faster. The million-order skewed and empty query ranges overlap. No percentile beyond the five-run median is inferred.

Root physical-plan statistics are incomplete. At one million orders, DataFusion's selective-join root estimates 20,000,000 rows for 10,000 actual rows, a Q-error of 2,000. optd's corresponding root statistic is absent. The scan returns one row; DataFusion reports Exact(1), while its AggregateExec mode Single in the optd plan reports Inexact(1,000,000), a Q-error of 1,000,000. This physical metadata differs from optd's own verbose estimate, which reports 1,000 rows for the global aggregate. The skewed join, grouped aggregate and empty roots have absent statistics on both sides. Missing values are not perfect estimates, and no overall cardinality-quality claim follows.

The complete million-order process peaked at 240,124 KiB RSS and exited zero. This resource number covers both sides and all queries, fixture setup and result conversion; it is not per-query or per-side peak memory. There are no service latency percentiles, disk-I/O results or general optimality claims in this in-memory experiment.

To reproduce the observed compiler configuration, build `cargo +1.99.0 build --release --locked -p optd-datafusion-bridge --example memoir --jobs 2`. Run `target/release/examples/memoir --benchmark evidence/benchmark/rows-100000 100000 5`, repeating at 1,000 and 1,000,000 orders. Run `python3 tools/verification/summarize_benchmark.py evidence/benchmark` to require all 30 groups and preserve medians, ranges, root estimate coverage and artifact hashes. Future compiler or configured-stack changes require matched comparisons on the same runner.
