#!/usr/bin/env bash
# Build the prepared source snapshot on the demonstration host.
set -euo pipefail
demo_root="${1:?usage: build-ec2.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
test -f "$demo_root/src/querygraph/Cargo.toml"
test -f "$demo_root/src/pinax/Cargo.toml"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_DEV_INCREMENTAL=false
export PATH="$demo_root/tools:$PATH"
export PYO3_PYTHON="${PYO3_PYTHON:-/usr/bin/python3}"
mkdir -p "$demo_root/logs" "$demo_root/reports"
rustc --version | tee "$demo_root/reports/rustc.txt"
cargo --version | tee "$demo_root/reports/cargo.txt"
cd "$demo_root/src/querygraph"
# Pinax 0.2.0 (`pinax-registry`) is published; the lockfile pins the reviewed registry checksum.
cargo build --locked \
  --bin querygraph --example pinax-client --example pinax-demo --example pinax-ontology-seed --example pinax-mcp-client --example pinax-delta-seed --example pinax-delta-read 2>&1 | tee "$demo_root/logs/querygraph-build.log"
sha256sum target/debug/querygraph > "$demo_root/reports/querygraph-binary.sha256"
cargo build --locked --manifest-path "$demo_root/src/sail/Cargo.toml" \
  --target-dir "$demo_root/src/lakecat/target" -p sail-cli --bin sail \
  2>&1 | tee "$demo_root/logs/sail-cli-build.log"
