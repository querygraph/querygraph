#!/usr/bin/env bash
set -euo pipefail

# Bootstrap a public QueryGraph demo on a fresh Debian EC2 host.
#
# Usage:
#   sudo bash bootstrap-debian-demo.sh
#
# Useful environment overrides:
#   QG_DEMO_ROOT=/opt/querygraph-demo
#   QG_DEMO_USER=qgdemo
#   QG_API_PORT=8080
#   QG_PUBLIC_PORT=80
#   QG_SAIL_PORT=50051
#   QG_BUILD_SAIL_FROM_SOURCE=1
#   QG_INSTALL_OLLAMA=0
#   QG_PULL_OLLAMA_MODEL=0
#   QG_OLLAMA_MODEL=llama3.2
#   QG_RUST_BRANCH=main
#   QG_PYTHON_BRANCH=main
#   QG_SAIL_BRANCH=grust
#   QG_GRUST_BRANCH=main
#   QG_LAKECAT_BRANCH=master
#   QG_TYPESEC_BRANCH=main
#   QG_FIRSTPAIR_BRANCH=main

log() {
  printf '\n[%s] %s\n' "$(date -Is)" "$*"
}

die() {
  printf '\nERROR: %s\n' "$*" >&2
  exit 1
}

need_root() {
  if [ "$(id -u)" -ne 0 ]; then
    die "run this script as root, for example: sudo bash $0"
  fi
}

run_as_demo() {
  su -s /bin/bash "$QG_DEMO_USER" -c "export HOME='$QG_DEMO_ROOT'; export PATH='$QG_DEMO_ROOT/.cargo/bin:/usr/local/bin:/usr/bin:/bin'; $*"
}

clone_or_update() {
  local url="$1"
  local dir="$2"
  local branch="$3"
  if [ -d "$dir/.git" ]; then
    log "Updating $dir"
    run_as_demo "cd '$dir' && git fetch --all --tags --prune && git checkout '$branch' && git pull --ff-only"
  else
    log "Cloning $url -> $dir ($branch)"
    install -d -o "$QG_DEMO_USER" -g "$QG_DEMO_USER" "$(dirname "$dir")"
    run_as_demo "git clone --branch '$branch' --depth 1 '$url' '$dir'"
  fi
}

write_file() {
  local path="$1"
  local mode="$2"
  local owner="$3"
  local group="$4"
  install -D -m "$mode" -o "$owner" -g "$group" /dev/stdin "$path"
}

need_root

export DEBIAN_FRONTEND=noninteractive

QG_DEMO_ROOT="${QG_DEMO_ROOT:-/opt/querygraph-demo}"
QG_DEMO_USER="${QG_DEMO_USER:-qgdemo}"
QG_API_PORT="${QG_API_PORT:-8080}"
QG_PUBLIC_PORT="${QG_PUBLIC_PORT:-80}"
QG_SAIL_PORT="${QG_SAIL_PORT:-50051}"
QG_BUILD_SAIL_FROM_SOURCE="${QG_BUILD_SAIL_FROM_SOURCE:-1}"
QG_INSTALL_OLLAMA="${QG_INSTALL_OLLAMA:-1}"
QG_PULL_OLLAMA_MODEL="${QG_PULL_OLLAMA_MODEL:-0}"
QG_OLLAMA_MODEL="${QG_OLLAMA_MODEL:-llama3.2}"

QG_REPO="${QG_REPO:-https://github.com/querygraph/querygraph.git}"
QG_SAIL_REPO="${QG_SAIL_REPO:-https://github.com/querygraph/sail.git}"
QG_GRUST_REPO="${QG_GRUST_REPO:-https://github.com/querygraph/grust.git}"
QG_LAKECAT_REPO="${QG_LAKECAT_REPO:-https://github.com/querygraph/lakecat.git}"
QG_TYPESEC_REPO="${QG_TYPESEC_REPO:-https://github.com/querygraph/typesec.git}"
QG_FIRSTPAIR_REPO="${QG_FIRSTPAIR_REPO:-https://github.com/firstpair/firstpair.git}"

QG_BRANCH="${QG_BRANCH:-main}"
QG_SAIL_BRANCH="${QG_SAIL_BRANCH:-grust}"
QG_GRUST_BRANCH="${QG_GRUST_BRANCH:-main}"
QG_LAKECAT_BRANCH="${QG_LAKECAT_BRANCH:-master}"
QG_TYPESEC_BRANCH="${QG_TYPESEC_BRANCH:-main}"
QG_FIRSTPAIR_BRANCH="${QG_FIRSTPAIR_BRANCH:-main}"

SRC_DIR="$QG_DEMO_ROOT/src"
BIN_DIR="$QG_DEMO_ROOT/bin"
RUN_DIR="$QG_DEMO_ROOT/run"
LOG_DIR="$QG_DEMO_ROOT/log"
WEB_DIR="$QG_DEMO_ROOT/web"

QG_DIR="$SRC_DIR/querygraph/querygraph"
QG_RUST_DIR="$QG_DIR"
QG_PYTHON_DIR="$QG_DIR/python"
QG_SAIL_DIR="$SRC_DIR/querygraph/sail"
QG_GRUST_DIR="$SRC_DIR/grust"
QG_LAKECAT_DIR="$SRC_DIR/lakecat"
QG_TYPESEC_DIR="$SRC_DIR/typesec"
QG_FIRSTPAIR_DIR="$SRC_DIR/firstpair"

log "Installing Debian packages"
apt-get update
apt-get install -y --no-install-recommends \
  bash ca-certificates curl git jq nginx pkg-config \
  build-essential clang cmake make protobuf-compiler \
  libssl-dev libsqlite3-dev python3 python3-dev python3-venv python3-pip \
  openjdk-17-jre-headless pandoc unzip zstd

if ! id "$QG_DEMO_USER" >/dev/null 2>&1; then
  log "Creating system user $QG_DEMO_USER"
  useradd --system --create-home --home-dir "$QG_DEMO_ROOT" --shell /bin/bash "$QG_DEMO_USER"
fi

install -d -o "$QG_DEMO_USER" -g "$QG_DEMO_USER" "$SRC_DIR" "$BIN_DIR" "$RUN_DIR" "$LOG_DIR" "$WEB_DIR"

log "Installing Rust toolchain for $QG_DEMO_USER"
if ! run_as_demo "test -x '$QG_DEMO_ROOT/.cargo/bin/cargo'"; then
  run_as_demo "curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal"
fi
run_as_demo "'$QG_DEMO_ROOT/.cargo/bin/rustup' default stable"
run_as_demo "'$QG_DEMO_ROOT/.cargo/bin/rustc' --version && '$QG_DEMO_ROOT/.cargo/bin/cargo' --version"

log "Installing uv"
if ! command -v uv >/dev/null 2>&1; then
  curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR=/usr/local/bin sh
fi
python3 - <<'PY'
import sys
if sys.version_info < (3, 11):
    raise SystemExit("Python 3.11+ is required; use Debian 12/bookworm or newer, or install python3.11 before running this script")
PY
uv --version

log "Cloning QueryGraph stack repositories"
clone_or_update "$QG_REPO" "$QG_DIR" "$QG_BRANCH"
clone_or_update "$QG_SAIL_REPO" "$QG_SAIL_DIR" "$QG_SAIL_BRANCH"
clone_or_update "$QG_GRUST_REPO" "$QG_GRUST_DIR" "$QG_GRUST_BRANCH"
clone_or_update "$QG_LAKECAT_REPO" "$QG_LAKECAT_DIR" "$QG_LAKECAT_BRANCH"
clone_or_update "$QG_TYPESEC_REPO" "$QG_TYPESEC_DIR" "$QG_TYPESEC_BRANCH"
clone_or_update "$QG_FIRSTPAIR_REPO" "$QG_FIRSTPAIR_DIR" "$QG_FIRSTPAIR_BRANCH"

log "Building QueryGraph Rust runtime"
run_as_demo "cd '$QG_RUST_DIR' && '$QG_DEMO_ROOT/.cargo/bin/cargo' build --release"
install -D -m 0755 -o root -g root "$QG_RUST_DIR/target/release/querygraph" "$BIN_DIR/querygraph"

if [ "$QG_BUILD_SAIL_FROM_SOURCE" = "1" ]; then
  log "Building Sail CLI from querygraph/sail ($QG_SAIL_BRANCH)"
  run_as_demo "cd '$QG_SAIL_DIR' && '$QG_DEMO_ROOT/.cargo/bin/cargo' build --release -p sail-cli"
  install -D -m 0755 -o root -g root "$QG_SAIL_DIR/target/release/sail" "$BIN_DIR/sail"
else
  log "Skipping local Sail source build; installing pysail CLI into Python environment instead"
fi

log "Syncing QueryGraph Python environment"
run_as_demo "cd '$QG_PYTHON_DIR' && uv sync --extra all"
if [ ! -x "$BIN_DIR/sail" ]; then
  run_as_demo "cd '$QG_PYTHON_DIR' && uv pip install pysail pyspark-client"
  write_file "$BIN_DIR/sail" 0755 root root <<EOF
#!/usr/bin/env bash
cd "$QG_PYTHON_DIR"
exec uv run sail "\$@"
EOF
fi

if [ "$QG_INSTALL_OLLAMA" = "1" ]; then
  log "Installing Ollama"
  if ! command -v ollama >/dev/null 2>&1; then
    curl -fsSL https://ollama.com/install.sh | sh
  fi
  systemctl enable --now ollama || true
  if [ "$QG_PULL_OLLAMA_MODEL" = "1" ]; then
    log "Pulling Ollama model $QG_OLLAMA_MODEL"
    ollama pull "$QG_OLLAMA_MODEL" || true
  fi
else
  log "Skipping Ollama install"
fi

log "Writing demo run scripts"
write_file "$BIN_DIR/querygraph-demo-smoke" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
curl -fsS http://127.0.0.1:$QG_API_PORT/v1/health | jq .
curl -fsS http://127.0.0.1:$QG_API_PORT/.well-known/agent-card.json | jq '.name, .skills'
curl -fsS http://127.0.0.1:$QG_API_PORT/v1/qglake/story | jq '{specialists: (.specialists | length), openlineage: .open_lineage.eventType}'
cd "$QG_RUST_DIR"
"$BIN_DIR/querygraph" dataverse-e2e \\
  --sail-dir "$RUN_DIR/sail" \\
  --openlineage-file "$RUN_DIR/openlineage/events.jsonl" \\
  --did-ledger-file "$RUN_DIR/did-ledger/attestations.jsonl" \\
  --question "Which governed datasets mention access control?" \\
  > "$RUN_DIR/dataverse-e2e-report.json"
jq '{datasets: (.datasets | length), access: .agentRun.access, eventType: .openLineage.event.eventType, attestation: .openLineage.attestation.signature[0:16]}' "$RUN_DIR/dataverse-e2e-report.json"
EOF

write_file "$BIN_DIR/querygraph-demo-seed-api" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$RUN_DIR"
for _ in \$(seq 1 60); do
  if curl -fsS http://127.0.0.1:$QG_API_PORT/v1/health >/dev/null; then
    break
  fi
  sleep 1
done
cd "$QG_RUST_DIR"
"$BIN_DIR/querygraph" navigator \\
  --dataset-name "Hazard vocabulary" \\
  --description "Controlled vocabulary with multilingual technical terms, access control, governance, and public safety labels" \\
  --landing-page "https://querygraph.ai/datasets/hazards" \\
  --data-url "https://querygraph.ai/datasets/hazards.csv" \\
  --creator "QueryGraph" \\
  --agent-name "AI Navigator" \\
  > "$RUN_DIR/demo-bundle.json"
jq '.layers.semanticCroissant' "$RUN_DIR/demo-bundle.json" \\
  | curl -fsS -X POST http://127.0.0.1:$QG_API_PORT/v1/models/import/croissant \\
      -H 'content-type: application/json' \\
      --data-binary @- \\
  | tee "$RUN_DIR/api-seed-result.json" >/dev/null
EOF

write_file "$BIN_DIR/querygraph-demo-live" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$QG_RUST_DIR"
"$BIN_DIR/querygraph" dataverse-e2e \\
  --sail-dir "$RUN_DIR/sail" \\
  --live-sail \\
  --sail-endpoint "http://127.0.0.1:$QG_SAIL_PORT" \\
  --openlineage-file "$RUN_DIR/openlineage/events.jsonl" \\
  --openlineage-sail-schema qg_audit \\
  --did-ledger-file "$RUN_DIR/did-ledger/attestations.jsonl" \\
  --question "Which governed datasets mention access control? Answer in one sentence." \\
  "\$@" \\
  > "$RUN_DIR/dataverse-e2e-live-report.json"
jq '{datasets: (.datasets | length), graph: .sail.graph, access: .agentRun.access, typedid: .agentRun.request.protocol, lineage: .openLineage.eventHash}' "$RUN_DIR/dataverse-e2e-live-report.json"
EOF

write_file "$BIN_DIR/querygraph-load-lakehouse" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$QG_RUST_DIR"
"$BIN_DIR/querygraph" lakehouse-load \\
  --root "$RUN_DIR/lakehouse" \\
  --schema qg_lakehouse \\
  --sail-endpoint "http://127.0.0.1:$QG_SAIL_PORT" \\
  "\$@" | tee "$RUN_DIR/lakehouse-load-summary.json"
"$BIN_DIR/querygraph" lakehouse-verify \\
  --report "$RUN_DIR/lakehouse/manifest/load-report.json" \\
  --sail-endpoint "http://127.0.0.1:$QG_SAIL_PORT" | tee "$RUN_DIR/lakehouse-verify.json"
"$BIN_DIR/querygraph" lakehouse-validate \\
  --report "$RUN_DIR/lakehouse/manifest/load-report.json" \\
  --openlineage-file "$RUN_DIR/openlineage/events.jsonl" | tee "$RUN_DIR/lakehouse-validate.json"
EOF

log "Writing Python MCP launcher"
write_file "$BIN_DIR/querygraph-mcp-http" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
cd "$QG_PYTHON_DIR"
exec uv run querygraph mcp-serve --transport streamable-http "\$@"
EOF

log "Writing documentation page builder"
write_file "$BIN_DIR/querygraph-build-docs-page" 0755 root root <<EOF
#!/usr/bin/env bash
set -euo pipefail
emitter="$QG_FIRSTPAIR_DIR/publishing/scripts/emit-html-book.sh"
docs_dir="$WEB_DIR/docs"
mkdir -p "\$docs_dir"

emit_book() {
  local repo="\$1"
  local title="\$2"
  local manuscript="\$3"
  local output_name="\$4"
  local dist_dir="\${5:-\$repo/docs/book/dist}"
  (
    cd "\$repo"
    BOOK_DIST_DIR="\$dist_dir" BOOK_HTML_MANUSCRIPT="\$manuscript" BOOK_VISIBLE_TITLE="\$title" "\$emitter"
  )
  cp "\$dist_dir/\${output_name}.html" "\$docs_dir/\${output_name}.html"
}

emit_book "$QG_GRUST_DIR" "Grust" "$QG_GRUST_DIR/docs/book/manuscript.md" "grust" "$QG_GRUST_DIR/docs/book/build/dist"
emit_book "$QG_TYPESEC_DIR" "Typesec" "$QG_TYPESEC_DIR/docs/book/typesec.md" "typesec"
emit_book "$QG_LAKECAT_DIR" "LakeCat" "$QG_LAKECAT_DIR/docs/book/lakecat.md" "lakecat"
emit_book "$QG_RUST_DIR" "Querygraph" "$QG_RUST_DIR/docs/book/manuscript.md" "querygraph"

cat > "\$docs_dir/index.html" <<'DOCS'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>QueryGraph Documentation Library</title>
  <style>
    body { margin: 0; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f7f8fb; color: #172026; }
    header { padding: 30px 34px 18px; background: #fff; border-bottom: 1px solid #d8dee8; }
    h1 { margin: 0 0 8px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #4e5b66; line-height: 1.45; }
    main { padding: 20px; display: grid; grid-template-columns: repeat(4, minmax(180px, 1fr)); gap: 14px; }
    a { display: block; min-height: 130px; padding: 16px; border: 1px solid #d8dee8; border-radius: 8px; background: #fff; color: #172026; text-decoration: none; }
    a:hover { border-color: #126b5b; }
    strong { display: block; font-size: 20px; margin-bottom: 8px; }
    span { color: #4e5b66; line-height: 1.4; }
    @media (max-width: 900px) { main { grid-template-columns: 1fr; } }
  </style>
</head>
<body>
  <header>
    <h1>QueryGraph Documentation Library</h1>
    <p>HTML editions generated from the release book sources.</p>
  </header>
  <main>
    <a href="./grust.html"><strong>Grust</strong><span>Rust property graph architecture and GQL/Cypher substrate.</span></a>
    <a href="./typesec.html"><strong>TypeSec</strong><span>Typed capabilities, receipts, and TypeDID governance.</span></a>
    <a href="./lakecat.html"><strong>LakeCat</strong><span>Engine-close catalog foundation for QueryGraph.</span></a>
    <a href="./querygraph.html"><strong>QueryGraph</strong><span>Governed semantic lakehouse for AI agents.</span></a>
  </main>
</body>
</html>
DOCS
EOF

log "Installing systemd services"
write_file /etc/systemd/system/querygraph-api.service 0644 root root <<EOF
[Unit]
Description=QueryGraph /v1 API
After=network-online.target
Wants=network-online.target

[Service]
User=$QG_DEMO_USER
WorkingDirectory=$QG_RUST_DIR
ExecStart=$BIN_DIR/querygraph serve --port $QG_API_PORT
ExecStartPost=$BIN_DIR/querygraph-demo-seed-api
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

write_file /etc/systemd/system/querygraph-sail.service 0644 root root <<EOF
[Unit]
Description=QueryGraph Sail Spark Connect server
After=network-online.target
Wants=network-online.target

[Service]
User=$QG_DEMO_USER
WorkingDirectory=$QG_RUST_DIR
ExecStart=$BIN_DIR/sail spark server --port $QG_SAIL_PORT
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

write_file /etc/systemd/system/querygraph-mcp.service 0644 root root <<EOF
[Unit]
Description=QueryGraph Python MCP HTTP server
After=network-online.target
Wants=network-online.target

[Service]
User=$QG_DEMO_USER
WorkingDirectory=$QG_PYTHON_DIR
ExecStart=$BIN_DIR/querygraph-mcp-http
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

log "Writing public demo page"
write_file "$WEB_DIR/index.html" 0644 "$QG_DEMO_USER" "$QG_DEMO_USER" <<'EOF'
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>QueryGraph Live Navigator</title>
  <style>
    :root { color-scheme: light; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    body { margin: 0; background: #f7f8fb; color: #172026; }
    header { padding: 28px 32px 18px; background: #ffffff; border-bottom: 1px solid #d8dee8; }
    h1 { margin: 0 0 8px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #4e5b66; line-height: 1.45; }
    main { display: grid; grid-template-columns: minmax(320px, 420px) minmax(0, 1fr); gap: 18px; padding: 18px; }
    section, aside { background: #ffffff; border: 1px solid #d8dee8; border-radius: 8px; padding: 16px; }
    label { display: block; font-size: 13px; font-weight: 650; margin: 14px 0 6px; color: #25313a; }
    textarea, input, select { box-sizing: border-box; width: 100%; border: 1px solid #b7c2cf; border-radius: 6px; padding: 10px; font: inherit; background: white; }
    textarea { min-height: 90px; resize: vertical; }
    button { margin-top: 14px; width: 100%; border: 0; border-radius: 6px; padding: 11px 14px; font-weight: 700; background: #126b5b; color: white; cursor: pointer; }
    button.secondary { background: #394b59; }
    button:disabled { opacity: .6; cursor: wait; }
    .grid { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 10px; margin-bottom: 12px; }
    .stat { border: 1px solid #d8dee8; border-radius: 8px; padding: 10px; background: #fbfcfe; }
    .stat b { display: block; font-size: 22px; }
    pre { overflow: auto; max-height: 620px; background: #101820; color: #d8f3dc; padding: 14px; border-radius: 8px; font-size: 12px; line-height: 1.45; }
    .tabs { display: flex; flex-wrap: wrap; gap: 8px; margin: 12px 0; }
    .tabs button { width: auto; margin: 0; background: #e8eef5; color: #18242e; }
    .tabs button.active { background: #126b5b; color: white; }
    @media (max-width: 900px) { main { grid-template-columns: 1fr; } .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  </style>
</head>
<body>
  <header>
    <h1>QueryGraph Live Navigator</h1>
    <p>Ask governed data questions and inspect the semantic bundle, policy receipts, TypeDID envelope, and OpenLineage evidence behind each answer.</p>
  </header>
  <main>
    <aside>
      <label for="question">Question</label>
      <textarea id="question">Which governed datasets mention access control?</textarea>
      <label for="persona">Persona</label>
      <select id="persona">
        <option>navigator agent</option>
        <option>public analyst</option>
        <option>restricted broker</option>
      </select>
      <button id="storyBtn">Run QGLake Story</button>
      <button id="bundleBtn" class="secondary">Build Navigator Bundle</button>
      <button id="answerBtn" class="secondary">Ask /v1/answer</button>
      <label for="endpoint">API endpoint</label>
      <input id="endpoint" value="/api">
    </aside>
    <section>
      <div class="grid">
        <div class="stat"><span>Health</span><b id="health">...</b></div>
        <div class="stat"><span>Specialists</span><b id="specialists">-</b></div>
        <div class="stat"><span>Lineage</span><b id="lineage">-</b></div>
        <div class="stat"><span>Access</span><b id="access">-</b></div>
      </div>
      <div class="tabs">
        <button class="active" data-view="summary">Summary</button>
        <button data-view="bundle">Bundle</button>
        <button data-view="openlineage">OpenLineage</button>
        <button data-view="typedid">TypeDID</button>
        <button data-view="raw">Raw</button>
      </div>
      <pre id="output">Loading health...</pre>
    </section>
  </main>
  <script>
    const output = document.getElementById("output");
    const endpoint = () => document.getElementById("endpoint").value.replace(/\/$/, "");
    const state = { story: null, bundle: null, answer: null, active: "summary" };
    const pretty = (value) => JSON.stringify(value, null, 2);
    async function get(path) {
      const res = await fetch(endpoint() + path);
      if (!res.ok) throw new Error(await res.text());
      return res.json();
    }
    async function post(path, body) {
      const res = await fetch(endpoint() + path, { method: "POST", headers: {"content-type": "application/json"}, body: JSON.stringify(body) });
      if (!res.ok) throw new Error(await res.text());
      return res.json();
    }
    function setBusy(button, busy) {
      button.disabled = busy;
      button.textContent = busy ? "Running..." : button.dataset.label;
    }
    function render() {
      const raw = state.answer || state.story || state.bundle || {};
      if (state.active === "bundle") output.textContent = pretty(state.bundle || raw.bundle || raw.layers || {});
      else if (state.active === "openlineage") output.textContent = pretty(raw.open_lineage || raw.openLineage || raw.openlineage || {});
      else if (state.active === "typedid") output.textContent = pretty(raw.envelope || raw.agentRun?.request || raw.synthesis?.request || {});
      else if (state.active === "raw") output.textContent = pretty(raw);
      else output.textContent = pretty({
        question: raw.question || document.getElementById("question").value,
        answer: raw.answer || raw.synthesis?.answer || raw.agentRun?.answer || "Run a demo action.",
        access: raw.agentRun?.access || raw.specialists?.map(s => ({agent: s.agent?.name, allowed: s.access?.allowed})),
        plans: raw.plans,
        matches: raw.matches
      });
    }
    async function refreshHealth() {
      try {
        const health = await get("/v1/health");
        document.getElementById("health").textContent = health.status;
        output.textContent = pretty(health);
      } catch (e) {
        document.getElementById("health").textContent = "down";
        output.textContent = String(e);
      }
    }
    async function runStory(e) {
      setBusy(e.target, true);
      try {
        state.story = await get("/v1/qglake/story");
        document.getElementById("specialists").textContent = state.story.specialists?.length ?? "-";
        document.getElementById("lineage").textContent = state.story.open_lineage?.eventType || "-";
        document.getElementById("access").textContent = "receipts";
        state.active = "summary";
        render();
      } finally { setBusy(e.target, false); }
    }
    async function buildBundle(e) {
      setBusy(e.target, true);
      try {
        state.bundle = await post("/v1/navigator/bundle", {
          dataset_name: "Hazard vocabulary",
          description: "Controlled vocabulary with multilingual technical terms",
          landing_page: "https://querygraph.ai/datasets/hazards",
          data_url: "https://querygraph.ai/datasets/hazards.csv",
          creator: "QueryGraph",
          agent_name: "AI Navigator"
        });
        state.active = "bundle";
        render();
      } finally { setBusy(e.target, false); }
    }
    async function askAnswer(e) {
      setBusy(e.target, true);
      try {
        state.answer = await post("/v1/answer", { question: document.getElementById("question").value });
        document.getElementById("lineage").textContent = state.answer.openlineage?.eventType || "-";
        document.getElementById("access").textContent = "signed";
        state.active = "summary";
        render();
      } finally { setBusy(e.target, false); }
    }
    document.querySelectorAll(".tabs button").forEach(button => {
      button.addEventListener("click", () => {
        document.querySelectorAll(".tabs button").forEach(b => b.classList.remove("active"));
        button.classList.add("active");
        state.active = button.dataset.view;
        render();
      });
    });
    for (const button of document.querySelectorAll("button")) button.dataset.label = button.textContent;
    document.getElementById("storyBtn").addEventListener("click", runStory);
    document.getElementById("bundleBtn").addEventListener("click", buildBundle);
    document.getElementById("answerBtn").addEventListener("click", askAnswer);
    refreshHealth();
  </script>
</body>
</html>
EOF

log "Configuring nginx"
write_file /etc/nginx/sites-available/querygraph-demo 0644 root root <<EOF
server {
    listen $QG_PUBLIC_PORT default_server;
    listen [::]:$QG_PUBLIC_PORT default_server;
    server_name _;

    root $WEB_DIR;
    index index.html;

    location / {
        try_files \$uri \$uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:$QG_API_PORT/;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location /.well-known/agent-card.json {
        proxy_pass http://127.0.0.1:$QG_API_PORT/.well-known/agent-card.json;
        proxy_set_header Host \$host;
    }

    location /mcp {
        proxy_pass http://127.0.0.1:8000/mcp;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location /docs/ {
        alias $WEB_DIR/docs/;
        index index.html;
    }
}
EOF
rm -f /etc/nginx/sites-enabled/default
ln -sf /etc/nginx/sites-available/querygraph-demo /etc/nginx/sites-enabled/querygraph-demo
nginx -t

log "Starting services"
systemctl daemon-reload
systemctl enable --now querygraph-api.service
systemctl enable --now querygraph-sail.service
systemctl enable --now querygraph-mcp.service
systemctl restart nginx

log "Waiting for QueryGraph API"
for _ in $(seq 1 60); do
  if curl -fsS "http://127.0.0.1:$QG_API_PORT/v1/health" >/dev/null; then
    break
  fi
  sleep 2
done
curl -fsS "http://127.0.0.1:$QG_API_PORT/v1/health" | jq .

log "Running smoke demo"
su -s /bin/bash "$QG_DEMO_USER" -c "$BIN_DIR/querygraph-demo-smoke"

log "Building documentation HTML page"
su -s /bin/bash "$QG_DEMO_USER" -c "$BIN_DIR/querygraph-build-docs-page"

cat <<EOF

QueryGraph demo is installed.

Public UI:
  http://<ec2-public-host>:$QG_PUBLIC_PORT/
Documentation:
  http://<ec2-public-host>:$QG_PUBLIC_PORT/docs/
MCP HTTP:
  http://<ec2-public-host>:$QG_PUBLIC_PORT/mcp

Local checks:
  $BIN_DIR/querygraph-demo-smoke
  $BIN_DIR/querygraph-demo-live
  $BIN_DIR/querygraph-load-lakehouse --max-files-per-dataset 1

Services:
  systemctl status querygraph-api
  systemctl status querygraph-sail
  systemctl status querygraph-mcp
  journalctl -u querygraph-api -f
  journalctl -u querygraph-sail -f

Artifacts:
  $RUN_DIR/dataverse-e2e-report.json
  $RUN_DIR/openlineage/events.jsonl
  $RUN_DIR/did-ledger/attestations.jsonl

Open EC2 inbound port $QG_PUBLIC_PORT to make the UI reachable.
EOF
