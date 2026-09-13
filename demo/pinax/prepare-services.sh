#!/usr/bin/env bash
# Create a retained synthetic Iceberg fixture once, before installing services.
set -euo pipefail
demo_root="${1:?usage: prepare-services.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
export PATH="$demo_root/tools:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false
export QG_FIXTURE_FETCH=1 QG_FIXTURE_BUILD_TIMEOUT=7200 QG_FIXTURE_PORT=18181
if test -e "$demo_root/run/pinax"; then
  echo 'Retained fixture already exists; refusing to replace its data.' >&2
  exit 1
fi
mkdir -p "$demo_root/run" "$demo_root/reports"
cd "$demo_root/src/querygraph"
uv sync --project python --extra mcp
uv run --project python --with 'pyiceberg[pyarrow,sql-sqlite]==0.10.0' \
  python integration/pinax/execute_rows.py \
  --lakecat-root "$demo_root/src/lakecat" --sail-root "$demo_root/src/sail" \
  --querygraph-bin "$demo_root/src/querygraph/target/debug/querygraph" \
  --prepare-demo "$demo_root/run/pinax" --report "$demo_root/reports/prepared-demo.json"
umask 077
test ! -e "$demo_root/run/owner.env"
printf 'LAKECAT_PLAN_TASK_SIGNING_KEY=%s\n' "$(openssl rand -hex 32)" > "$demo_root/run/owner.env"

"$demo_root/src/querygraph/target/debug/examples/pinax-ontology-seed" \
  --registry "$demo_root/run/pinax/registry.json" \
  --store "$demo_root/run/ontology" \
  --config "$demo_root/run/pinax/registry-service.json" \
  > "$demo_root/reports/ontology-seed.json"

# Preserve the original fixture paths; activate the reviewed business inventory separately.
uv run --project python --with 'pyiceberg[pyarrow,sql-sqlite]==0.10.0' \
  python integration/pinax/prepare_customers.py \
  --existing "$demo_root/run/pinax" --destination "$demo_root/run/customer-discovery" \
  --querygraph-bin "$demo_root/src/querygraph/target/debug/querygraph" \
  > "$demo_root/reports/customer-prepared.json"
"$demo_root/src/querygraph/target/debug/examples/pinax-ontology-seed" \
  --registry "$demo_root/run/customer-discovery/registry.json" \
  --store "$demo_root/run/customer-ontology" \
  --config "$demo_root/run/customer-discovery/registry-service.json" \
  > "$demo_root/reports/customer-ontology.json"
ln -s customer-discovery "$demo_root/run/active-pinax"
