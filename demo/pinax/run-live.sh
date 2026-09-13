#!/usr/bin/env bash
set -euo pipefail
demo_root="${1:?usage: run-live.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
fixture_root="$demo_root/run/pinax"
if test -e "$demo_root/run/active-pinax"; then fixture_root="$demo_root/run/active-pinax"; fi
querygraph="$demo_root/src/querygraph/target/debug/querygraph"
client="$demo_root/src/querygraph/target/debug/examples/pinax-client"
config="$fixture_root/registry-service.json"
report_dir="$(mktemp -d "$demo_root/reports/live-XXXXXXXX")"
case "${2:-}" in
  '')
    for operation in lakehouse discover ontology mcp; do
      curl --fail --silent --show-error --max-time 125 \
        -X POST -H 'x-querygraph-demo: 1' \
        "http://127.0.0.1:18081/api/run/$operation" \
        > "$report_dir/$operation.json"
    done
    printf 'Part I customer discovery reports: %s\n' "$report_dir"
    exit 0
    ;;
  --part-ii) ;;
  *) echo 'usage: run-live.sh DEMO_ROOT [--part-ii]' >&2; exit 2 ;;
esac
export QG_SAIL_WAREHOUSE="$report_dir/warehouse"
"$querygraph" dataverse-e2e --sail-dir "$report_dir/sail" \
  --live-sail --sail-endpoint http://127.0.0.1:15051 \
  --openlineage-file "$report_dir/events.jsonl" \
  --did-ledger-file "$report_dir/attestations.jsonl" \
  --question 'Which governed datasets mention access control?' \
  > "$report_dir/semantic.json"
for operation in ontology discover plan execute; do
  "$client" --config "$config" "$operation" > "$report_dir/pinax-$operation.json"
done
"$demo_root/src/querygraph/target/debug/examples/pinax-mcp-client" \
  --url http://127.0.0.1:18082/mcp > "$report_dir/mcp-stateless.json"
for operation in deny-column deny-purpose; do
  if "$client" --config "$config" "$operation" > "$report_dir/$operation.stdout" 2> "$report_dir/$operation.stderr"; then
    echo "Expected $operation to fail" >&2
    exit 1
  fi
  test ! -s "$report_dir/$operation.stdout"
  grep -Fq 'Pinax scan denied' "$report_dir/$operation.stderr"
done
curl --fail --silent http://127.0.0.1:18080/v1/health > "$report_dir/health.json"
curl --fail --silent http://127.0.0.1:18080/v1/qglake/story > "$report_dir/qglake-story.json"
printf 'Live stack reports: %s\n' "$report_dir"
