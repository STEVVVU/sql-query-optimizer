"""Summarize retained paired samples without selecting or dropping regressions."""
import argparse
import csv
import hashlib
import json
from pathlib import Path
import re
import statistics


def q_error(estimate, actual):
    match = re.fullmatch(r"(?:Exact|Inexact)\((\d+)\)", estimate)
    if not match:
        return None
    predicted = int(match[1])
    if actual == 0 or predicted == 0:
        return 1.0 if actual == predicted else None
    return max(predicted/actual, actual/predicted)


def summarize(rows):
    groups = {}
    for row in rows:
        if row["warmup"] == "true":
            continue
        key = (row["orders"], row["query"], row["side"])
        groups.setdefault(key, []).append(row)
    output = []
    for (orders, query, side), samples in groups.items():
        item = {"orders":int(orders),"query":query,"side":side,"n":len(samples),"actual_rows":int(samples[0]["rows"]),"estimated_root_rows":samples[0]["estimated_root_rows"],"root_q_error":q_error(samples[0]["estimated_root_rows"],int(samples[0]["rows"]))}
        for field in ["planning_us","execution_us","total_us"]:
            values = [int(s[field]) for s in samples]
            item[field] = {"median":statistics.median(values),"min":min(values),"max":max(values)}
        output.append(item)
    return output


def validate_complete(rows, sizes=(1000,100000,1000000), repeats=5, hybrid=False):
    sides = ["optd","datafusion","optd_df_logical"] if hybrid else ["optd","datafusion"]
    expected = {(str(size),query,side) for size in sizes for query in ["skewed_join","aggregate","selective_join","scan","empty"] for side in sides}
    groups = {}
    for row in rows:
        key = (row["orders"],row["query"],row["side"])
        groups.setdefault(key,[]).append(row)
    if set(groups) != expected:
        raise ValueError(f"incomplete or unexpected groups: expected {len(expected)}, got {len(groups)}")
    for key,samples in groups.items():
        warmups = [s for s in samples if s["warmup"] == "true"]
        measured = [s for s in samples if s["warmup"] == "false"]
        if len(warmups) != 1 or len(measured) != repeats or {int(s["repeat"]) for s in measured} != set(range(1,repeats+1)):
            raise ValueError(f"incomplete or duplicate repeats: {key}")
        if len({s["rows"] for s in samples}) != 1:
            raise ValueError(f"row count drift: {key}")


assert q_error("Exact(10)",5) == 2
assert q_error("Inexact(0)",0) == 1
assert q_error("Absent",5) is None
assert q_error("Exact(0)",5) is None
try:
    validate_complete([])
    raise AssertionError("missing samples accepted")
except ValueError:
    pass

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--hybrid", action="store_true")
    args = parser.parse_args()
    root = args.root
    rows = []
    hashes = {}
    for path in sorted(root.glob("rows-*/samples.csv")):
        if not (path.parent/"passed.txt").is_file():
            raise RuntimeError(f"missing correctness marker for {path}")
        data = list(csv.DictReader(path.open()))
        rows.extend(data)
        for artifact in path.parent.iterdir():
            if artifact.is_file():
                hashes[str(artifact.relative_to(root))] = hashlib.sha256(artifact.read_bytes()).hexdigest()
    if not rows:
        raise RuntimeError("no benchmark samples")
    validate_complete(rows, hybrid=args.hybrid)
    result = {"hybrid_arm":args.hybrid,"summary":summarize(rows),"artifact_sha256":hashes,"policy":"All expected groups (30 standalone, 45 with explicitly requested hybrid arm) require exactly five measured repeats plus one warmup. All samples retained. Warmup excluded. Root plan estimates are distinct from optd internal estimates in verbose EXPLAIN. Q-error is undefined for nonmatching zero estimates/actuals or absent statistics. No speedup verdict without separate source-level profile and generator/resource review."}
    (root/"summary.json").write_text(json.dumps(result,indent=2)+"\n")
