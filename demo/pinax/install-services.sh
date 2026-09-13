#!/usr/bin/env bash
# Install only this demo's named services; keep fixture backends on loopback.
set -euo pipefail
demo_root="${1:?usage: install-services.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
[[ "$demo_root" =~ ^/[A-Za-z0-9_./-]+$ ]] || { echo 'Unsupported service path' >&2; exit 1; }
demo_user="$(id -un)"
querygraph="$demo_root/src/querygraph/target/debug/querygraph"
sail="$demo_root/src/lakecat/target/debug/sail"
owner="$demo_root/src/lakecat/target/debug/querygraph-registry-live-fixture"
console="$demo_root/src/querygraph/target/debug/examples/pinax-demo"
for binary in "$querygraph" "$sail" "$owner" "$console"; do test -x "$binary"; done
test -f "$demo_root/run/pinax/registry-service.json"
test -f "$demo_root/run/owner.env"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

write_unit() {
  local name="$1" description="$2" command="$3" environment="$4"
  cat > "$scratch/$name.service" <<EOF
[Unit]
Description=$description
After=network.target

[Service]
Type=simple
User=$demo_user
WorkingDirectory=$demo_root
UMask=0077
$environment
ExecStart=$command
Restart=on-failure
RestartSec=3
KillSignal=SIGINT
TimeoutStopSec=15

[Install]
WantedBy=multi-user.target
EOF
}

write_unit querygraph-pinax-sail 'QueryGraph demo Spark Connect' \
  "$sail spark server --ip 127.0.0.1 --port 15051" \
  "Environment=PYTHONPATH=$demo_root/src/sail/python"
write_unit querygraph-pinax-owner 'QueryGraph demo governed Iceberg owner' \
  "$owner $demo_root/run/pinax" \
  "Environment=QG_FIXTURE_PORT=18181
EnvironmentFile=$demo_root/run/owner.env"
write_unit querygraph-pinax-api 'QueryGraph signed semantic HTTP API' \
  "$querygraph serve --require-auth --port 18080" \
  "Environment=QG_SAIL_WAREHOUSE=$demo_root/run/semantic-warehouse"
write_unit querygraph-pinax-console 'QueryGraph Pinax demonstration console' \
  "$console --demo-root $demo_root --port 18081" ""
write_unit querygraph-pinax-mcp 'QueryGraph stateless MCP and central ontology' \
  "$querygraph mcp-serve --listen 127.0.0.1:18082 --registry-config $demo_root/run/pinax/registry-service.json" ""

for name in querygraph-pinax-sail querygraph-pinax-owner querygraph-pinax-api querygraph-pinax-console querygraph-pinax-mcp; do
  if systemctl cat "$name.service" > "$scratch/existing" 2>/dev/null; then
    grep -Fq "$demo_root/" "$scratch/existing" || { echo "Refusing unrelated $name" >&2; exit 1; }
    sudo -n systemctl stop "$name.service"
  fi
  sudo -n install -m 0644 "$scratch/$name.service" "/etc/systemd/system/$name.service"
done
sudo -n systemctl daemon-reload
sudo -n systemctl enable --now querygraph-pinax-sail querygraph-pinax-owner querygraph-pinax-api querygraph-pinax-console querygraph-pinax-mcp
systemctl is-active querygraph-pinax-sail querygraph-pinax-owner querygraph-pinax-api querygraph-pinax-console querygraph-pinax-mcp
bash "$demo_root/src/querygraph/demo/pinax/wait-services.sh" "$demo_root"
