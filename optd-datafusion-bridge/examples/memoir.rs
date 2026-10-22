//! Run SQL through optd and DataFusion, retaining memo and plan diagnostics.
#![deny(clippy::all)]
use std::{path::PathBuf, sync::Arc};

use anyhow::{ensure, Result};
use datafusion::{
    arrow::{
        array::{ArrayRef, Int64Array, StringArray},
        record_batch::RecordBatch,
        util::pretty::pretty_format_batches,
    },
    datasource::MemTable,
    prelude::SessionContext,
};
use optd_datafusion_bridge::create_df_context;

fn rows(batches: &[RecordBatch]) -> Result<Vec<Vec<String>>> {
    use datafusion::arrow::util::display::{ArrayFormatter, FormatOptions};
    let options = FormatOptions::default().with_null("NULL");
    let mut rows = Vec::new();
    for batch in batches {
        let columns = batch
            .columns()
            .iter()
            .map(|column| ArrayFormatter::try_new(column.as_ref(), &options))
            .collect::<Result<Vec<_>, _>>()?;
        for row in 0..batch.num_rows() {
            rows.push(
                columns
                    .iter()
                    .map(|column| column.value(row).to_string())
                    .collect(),
            );
        }
    }
    Ok(rows)
}

fn table(columns: Vec<(&str, Vec<i64>)>) -> Result<Arc<MemTable>> {
    let batch = RecordBatch::try_from_iter(
        columns
            .into_iter()
            .map(|(name, values)| (name, Arc::new(Int64Array::from(values)) as ArrayRef)),
    )?;
    Ok(Arc::new(MemTable::try_new(
        batch.schema(),
        vec![vec![batch]],
    )?))
}

fn seed(ctx: &SessionContext, order_count: i64) -> Result<()> {
    ctx.register_table(
        "customers",
        table(vec![
            ("id", (0..20).collect()),
            ("region_id", (0..20).map(|i| i % 4).collect()),
        ])?,
    )?;
    ctx.register_table(
        "orders",
        table(vec![
            ("id", (0..order_count).collect()),
            (
                "customer_id",
                (0..order_count)
                    .map(|i| if i < order_count * 8 / 10 { 0 } else { i % 20 })
                    .collect(),
            ),
            ("amount", (0..order_count).map(|i| i % 100).collect()),
        ])?,
    )?;
    ctx.register_table(
        "regions",
        table(vec![("id", (0..4).collect()), ("active", vec![1, 0, 1, 0])])?,
    )?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("--benchmark") {
        return benchmark().await;
    }
    let evidence = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "evidence/memoir".into()),
    );
    std::fs::create_dir_all(&evidence)?;
    let summary = evidence.join("summary.txt");
    if summary.exists() {
        std::fs::remove_file(&summary)?;
    }
    let optd = create_df_context(None, None, None, false, false, false, None).await?;
    let reference = SessionContext::new();
    seed(&optd.ctx, 1000)?;
    seed(&reference, 1000)?;
    let queries = [
        ("skewed_join", "SELECT o.id, c.id AS customer, r.id AS region FROM orders o JOIN customers c ON o.customer_id = c.id JOIN regions r ON c.region_id = r.id WHERE r.active = 1 AND o.amount > 90 ORDER BY o.id"),
        ("aggregate", "SELECT c.region_id, COUNT(*) AS n, SUM(o.amount) AS total FROM orders o JOIN customers c ON o.customer_id = c.id JOIN regions r ON c.region_id = r.id WHERE r.active = 1 GROUP BY c.region_id ORDER BY c.region_id"),
        ("empty", "SELECT o.id FROM orders o JOIN customers c ON o.customer_id = c.id JOIN regions r ON c.region_id = r.id WHERE o.amount > 1000 ORDER BY o.id"),
    ];
    for (name, sql) in queries {
        std::fs::write(evidence.join(format!("{name}.sql")), sql)?;
        let actual = optd.ctx.sql(sql).await?.collect().await?;
        let expected = reference.sql(sql).await?.collect().await?;
        if let (Some(actual), Some(expected)) = (actual.first(), expected.first()) {
            ensure!(
                actual
                    .schema()
                    .fields()
                    .iter()
                    .map(|field| field.data_type())
                    .eq(expected
                        .schema()
                        .fields()
                        .iter()
                        .map(|field| field.data_type())),
                "{name}: physical output types differ"
            );
        }
        ensure!(
            rows(&actual)? == rows(&expected)?,
            "{name}: row values differ from reference"
        );
        let expected_count = match name {
            "skewed_join" => 80,
            "aggregate" => 2,
            _ => 0,
        };
        ensure!(
            rows(&actual)?.len() == expected_count,
            "{name}: fixture row count differs"
        );
        let actual = pretty_format_batches(&actual)?.to_string();
        let expected = pretty_format_batches(&expected)?.to_string();
        std::fs::write(evidence.join(format!("{name}.reference.txt")), &expected)?;
        std::fs::write(evidence.join(format!("{name}.results.txt")), &actual)?;
        let explain = optd
            .ctx
            .sql(&format!("EXPLAIN VERBOSE {sql}"))
            .await?
            .collect()
            .await?;
        let mut plan = String::new();
        let mut join_alternatives = 0;
        for batch in &explain {
            let kinds = batch
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let plans = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            for row in 0..batch.num_rows() {
                plan.push_str(&format!("{}\n{}\n\n", kinds.value(row), plans.value(row)));
                if kinds.value(row).contains("optd-all-logical-join-orders") {
                    join_alternatives = plans.value(row).lines().count();
                }
            }
        }
        std::fs::write(evidence.join(format!("{name}.plan.txt")), &plan)?;
        let guard = optd.optimizer.optimizer.lock().unwrap();
        let cascades = guard.as_ref().unwrap().optd_cascades_optimizer();
        let mut memo = String::new();
        cascades.dump(&mut memo)?;
        std::fs::write(evidence.join(format!("{name}.memo.txt")), &memo)?;
        ensure!(
            memo.contains("weighted_cost="),
            "missing costed memo winners"
        );
        ensure!(
            plan.contains("PhysicalHashJoin"),
            "missing physical hash join"
        );
        if name == "skewed_join" {
            ensure!(
                join_alternatives > 1,
                "join exploration produced no alternatives"
            );
            ensure!(plan.contains("PhysicalFilter"), "missing pushed filter");
            let lines = plan.lines().collect::<Vec<_>>();
            let scan_filters = lines
                .windows(2)
                .filter(|pair| {
                    pair[0].trim().starts_with("FilterExec:")
                        && pair[1].trim().starts_with("MemoryExec:")
                })
                .count();
            ensure!(
                scan_filters == 2,
                "expected both filters directly above table scans"
            );
        }
        println!("PASS {name}: SQL result parity; {join_alternatives} logical join orders; costed memo retained");
    }
    std::fs::write(evidence.join("summary.txt"), "PASS: 3 SQL queries match DataFusion reference; plans and costed memo retained.\nFixture: orders=1000 (80% customer 0), customers=20, regions=4.\nNo timing or cost-improvement claim; default cost model uses estimates.\n")?;
    Ok(())
}

async fn benchmark() -> Result<()> {
    use datafusion::physical_plan::{collect, display::DisplayableExecutionPlan};
    use datafusion::prelude::SessionConfig;
    use std::io::Write;
    use std::time::Instant;
    let args = std::env::args().collect::<Vec<_>>();
    ensure!(
        args.len() == 5,
        "usage: memoir --benchmark OUTPUT ORDERS REPEATS"
    );
    let evidence = PathBuf::from(&args[2]);
    let count = args[3].parse::<i64>()?;
    let repeats = args[4].parse::<usize>()?;
    ensure!(
        (1..=1_000_000).contains(&count),
        "orders must be 1..1000000"
    );
    ensure!(
        (1..=20).contains(&repeats),
        "repeats must be 1..20; use at least five for reported comparisons"
    );
    ensure!(!evidence.exists(), "evidence directory must be new");
    std::fs::create_dir_all(&evidence)?;
    let queries = [
        ("skewed_join", "SELECT o.id, c.id AS customer, r.id AS region FROM orders o JOIN customers c ON o.customer_id = c.id JOIN regions r ON c.region_id = r.id WHERE r.active = 1 AND o.amount > 90 ORDER BY o.id"),
        ("aggregate", "SELECT c.region_id, COUNT(*) AS n, SUM(o.amount) AS total FROM orders o JOIN customers c ON o.customer_id = c.id JOIN regions r ON c.region_id = r.id WHERE r.active = 1 GROUP BY c.region_id ORDER BY c.region_id"),
        ("selective_join", "SELECT o.id FROM orders o JOIN customers c ON o.customer_id = c.id WHERE c.id = 1 ORDER BY o.id"),
        ("scan", "SELECT SUM(amount) AS total, COUNT(*) AS n FROM orders"),
        ("empty", "SELECT o.id FROM orders o JOIN customers c ON o.customer_id = c.id WHERE o.amount > 1000 ORDER BY o.id"),
    ];
    let mut samples = std::fs::File::create(evidence.join("samples.csv"))?;
    writeln!(samples, "query,side,repeat,warmup,orders,planning_us,execution_us,total_us,rows,estimated_root_rows")?;
    for (name, sql) in queries {
        std::fs::write(evidence.join(format!("{name}.sql")), sql)?;
        for repeat in 0..=repeats {
            let mut sides = ["optd", "datafusion"];
            if repeat % 2 == 1 {
                sides.reverse();
            }
            let mut expected = None;
            for side in sides {
                let config = SessionConfig::new()
                    .with_target_partitions(4)
                    .with_batch_size(8192);
                let optd = if side == "optd" {
                    Some(
                        create_df_context(
                            Some(config.clone()),
                            None,
                            None,
                            false,
                            false,
                            false,
                            None,
                        )
                        .await?,
                    )
                } else {
                    None
                };
                let reference = SessionContext::new_with_config(config);
                let ctx = optd.as_ref().map_or(&reference, |optd| &optd.ctx);
                seed(ctx, count)?;
                let start = Instant::now();
                let plan = ctx.sql(sql).await?.create_physical_plan().await?;
                let planning = start.elapsed().as_micros();
                let execute = Instant::now();
                let actual = collect(plan.clone(), ctx.task_ctx()).await?;
                let execution = execute.elapsed().as_micros();
                let total = start.elapsed().as_micros();
                let values = rows(&actual)?;
                if let Some(expected) = &expected {
                    ensure!(
                        &values == expected,
                        "{name}: result mismatch on repeat {repeat}"
                    );
                } else {
                    expected = Some(values.clone());
                }
                let row_count = values.len();
                let estimate = format!("{:?}", plan.statistics()?.num_rows);
                writeln!(samples, "{name},{side},{repeat},{},{count},{planning},{execution},{total},{row_count},{estimate}", repeat == 0)?;
                samples.flush()?;
                if repeat == 0 {
                    std::fs::write(
                        evidence.join(format!("{name}.{side}.plan.txt")),
                        DisplayableExecutionPlan::with_metrics(plan.as_ref())
                            .indent(true)
                            .to_string(),
                    )?;
                    std::fs::write(
                        evidence.join(format!("{name}.{side}.rows.txt")),
                        format!("{values:?}"),
                    )?;
                    if let Some(optd) = &optd {
                        let explain = optd
                            .ctx
                            .sql(&format!("EXPLAIN VERBOSE {sql}"))
                            .await?
                            .collect()
                            .await?;
                        std::fs::write(
                            evidence.join(format!("{name}.optd.explain.txt")),
                            pretty_format_batches(&explain)?.to_string(),
                        )?;
                    }
                }
            }
        }
        println!(
            "PASS benchmark {name} orders={count} repeats={repeats}: identical typed row values"
        );
    }
    std::fs::write(evidence.join("passed.txt"), "All paired results matched. Warmup is repeat 0. Fixture construction and result conversion are outside timers. Planning plus execution is the in-memory consumer path.\n")?;
    Ok(())
}
