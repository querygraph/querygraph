#!/usr/bin/env bash
# Package the inspected source snapshot; runtime data and credentials stay out.
set -euo pipefail
demo_root="${1:?usage: package-ec2.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
mkdir -p "$demo_root/artifacts"
scratch="$(mktemp -d "$demo_root/package-XXXXXXXX")"
trap 'rm -rf "$scratch"' EXIT
mkdir "$scratch/src"
rsync -a --exclude=target --exclude=.git --exclude=.venv --exclude=node_modules \
  --exclude=__pycache__ --exclude=.pytest_cache --exclude=.ruff_cache \
  --exclude='.env' --exclude='.env.*' --exclude='*.pem' --exclude='*.key' \
  --exclude='*.textpack' --exclude='*.tar.gz' --exclude='*.zip' \
  --exclude='/querygraph/sail' "$demo_root/src/" "$scratch/src/"
cd "$scratch"
find src -type f -print0 | sort -z | xargs -0 sha256sum > SOURCE-SHA256SUMS
cp src/querygraph/demo/pinax/evidence/source-provenance.json PROVENANCE.json
for executable in \
  src/querygraph/target/debug/querygraph \
  src/querygraph/target/debug/examples/pinax-client \
  src/querygraph/target/debug/examples/pinax-demo \
  src/querygraph/target/debug/examples/pinax-ontology-seed \
  src/querygraph/target/debug/examples/pinax-mcp-client \
  src/querygraph/target/debug/examples/pinax-delta-seed \
  src/querygraph/target/debug/examples/pinax-delta-read \
  src/lakecat/target/debug/sail \
  src/lakecat/target/debug/querygraph-registry-live-fixture; do
  (cd "$demo_root" && sha256sum "$executable")
done > BUILT-BINARY-SHA256SUMS
cp src/querygraph/demo/pinax/README.md README.md
cat > START-HERE.txt <<'EOF'
This is a prepared-source QueryGraph stack demo with Pinax.
The source file checksums identify the exact packaged snapshot; the binary
checksums identify the demonstrated EC2 build, not portable supplied binaries.

On a Debian host with Rust 2024 support, matching Python development libraries,
uv, build-essential, clang, cmake, protobuf-compiler, pkg-config, libssl-dev,
curl, openssl and passwordless sudo for systemd installation:

  sha256sum -c SOURCE-SHA256SUMS
  bash src/querygraph/demo/pinax/deploy.sh "$(pwd)"

The deployment compiles sources on the destination. It does not contain Cargo
registry caches, SSH credentials, runtime signing keys, data or virtualenvs.
Dependency downloads require network access. Pinax is the released
pinax-registry 0.2.0 dependency. The LakeCat and Sail fixtures use the included
source checkouts, independently of upstream Sail review or merging.
EOF
tar -czf "$demo_root/artifacts/querygraph-pinax-ec2.tar.gz" .
cd "$demo_root/artifacts"
sha256sum querygraph-pinax-ec2.tar.gz > querygraph-pinax-ec2.tar.gz.sha256
cat querygraph-pinax-ec2.tar.gz.sha256
