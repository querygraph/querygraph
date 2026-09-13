#!/usr/bin/env bash
set -euo pipefail
demo_root="${1:?usage: deploy.sh DEMO_ROOT}"
demo_root="$(cd "$demo_root" && pwd)"
scripts="$demo_root/src/querygraph/demo/pinax"
bash "$scripts/build-ec2.sh" "$demo_root"
bash "$scripts/run-ec2.sh" "$demo_root"
bash "$scripts/prepare-services.sh" "$demo_root"
bash "$scripts/install-services.sh" "$demo_root"
bash "$scripts/run-live.sh" "$demo_root"
printf 'Open an SSH tunnel for port 18081, then visit http://localhost:18081\n'
