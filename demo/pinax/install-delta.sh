#!/usr/bin/env bash
# Add the second read-only owner and MCP endpoint without restarting Iceberg.
set -euo pipefail
demo_root="${1:?usage: install-delta.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
[[ "$demo_root" =~ ^/[A-Za-z0-9_./-]+$ ]] || exit 1
querygraph="$demo_root/src/querygraph/target/debug"
fixture="$demo_root/run/customer-delta"
if ! test -e "$fixture"; then
  "$querygraph/examples/pinax-delta-seed" --existing "$demo_root/run/active-pinax" --destination "$fixture" > "$demo_root/reports/delta-seed.json"
fi
test -f "$fixture/registry-service.json"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
for kind in owner mcp; do
  if test "$kind" = owner; then
    command="$demo_root/src/lakecat/target/debug/querygraph-registry-live-fixture $fixture"
    environment="Environment=QG_FIXTURE_PORT=18182
Environment=QG_DELTA_READER=$querygraph/examples/pinax-delta-read
EnvironmentFile=$demo_root/run/owner.env"
  else
    command="$querygraph/querygraph mcp-serve --listen 127.0.0.1:18084 --registry-config $fixture/registry-service.json"
    environment=""
  fi
  cat > "$scratch/querygraph-pinax-delta-$kind.service" <<UNIT
[Unit]
Description=QueryGraph Delta Lake demo $kind
After=network.target querygraph-pinax-sail.service
[Service]
Type=simple
User=$(id -un)
WorkingDirectory=$demo_root
UMask=0077
$environment
ExecStart=$command
Restart=on-failure
RestartSec=3
[Install]
WantedBy=multi-user.target
UNIT
  sudo -n install -m 644 "$scratch/querygraph-pinax-delta-$kind.service" /etc/systemd/system/
done
sudo -n systemctl daemon-reload
sudo -n systemctl enable --now querygraph-pinax-delta-owner querygraph-pinax-delta-mcp
