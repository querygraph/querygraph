#!/usr/bin/env bash
# Retain the original semantic workflow and add real Pinax execution.
set -euo pipefail
demo_root="${1:?usage: run-ec2.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
export PATH="$demo_root/tools:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_DEV_INCREMENTAL=false
export QG_FIXTURE_FETCH=1
export QG_FIXTURE_BUILD_TIMEOUT=7200
querygraph="$demo_root/src/querygraph/target/debug/querygraph"
test -x "$querygraph"
report_dir="$(mktemp -d "$demo_root/reports/run-XXXXXXXX")"
cd "$demo_root/src/querygraph"
if [[ "${2:-}" == "--part-ii" ]]; then
  "$querygraph" dataverse-e2e --sail-dir "$report_dir/sail" \
  --openlineage-file "$report_dir/openlineage.jsonl" \
  --did-ledger-file "$report_dir/attestations.jsonl" \
  --question "Which governed datasets mention access control?" \
  > "$report_dir/dataverse-e2e.json"
fi
uv sync --project python --extra mcp
uv run --project python --with 'pyiceberg[pyarrow,sql-sqlite]==0.10.0' \
  python integration/pinax/execute_rows.py \
  --lakecat-root "$demo_root/src/lakecat" --sail-root "$demo_root/src/sail" \
  --querygraph-bin "$querygraph" --report "$report_dir/pinax-execution.json" \
  2>&1 | tee "$report_dir/pinax-execution.log"
printf 'Reports: %s\n' "$report_dir"
