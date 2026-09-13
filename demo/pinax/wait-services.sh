#!/usr/bin/env bash
set -euo pipefail
demo_root="${1:?usage: wait-services.sh DEMO_ROOT}"
client="$demo_root/src/querygraph/target/debug/examples/pinax-client"
for attempt in {1..30}; do
  if curl --fail --silent --max-time 1 http://127.0.0.1:18080/v1/health > /dev/null \
    && curl --fail --silent --max-time 1 http://127.0.0.1:18081/ > /dev/null \
    && curl --fail --silent --max-time 1 http://127.0.0.1:18082/mcp \
      -H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream' \
      -H 'MCP-Protocol-Version: 2026-07-28' -H 'Mcp-Method: server/discover' \
      --data '{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}' > /dev/null \
    && (exec 3<>/dev/tcp/127.0.0.1/15051) 2>/dev/null \
    && "$client" --config "$demo_root/run/pinax/registry-service.json" discover \
      > /dev/null 2> "$demo_root/logs/owner-startup.log"; then
    exit 0
  fi
  sleep 1
done
echo 'Demo services did not become ready; inspect systemd and logs/owner-startup.log.' >&2
exit 1
