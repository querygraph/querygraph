# QueryGraph stack dependency graph

This file is the single place where the QueryGraph family records **which
released version of each sibling every other sibling depends on**. It exists
because the 2026-09-04 alignment release found four stale pins at once —
TypeSec, Marciana, LakeCat, and QueryGraph each tracked an older Grust or
TypeSec line, and Marciana's `lakecat-core 0.3.0` pin silently gave
QueryGraph two incompatible copies of the governed-scan types. A stale pin is
not a cosmetic lag: in `0.x` SemVer every minor is a breaking line, so a
consumer that pins `^0.13.0` cannot resolve a sibling's `0.14.0` and the
stack quietly forks.

## The rule

1. **Release order is the graph order.** `grust` → `typesec` → `marciana` →
   `lakecat` → `querygraph`. A release of any repository is followed by a
   release of every repository downstream of it in this order, each bumping
   its pins to the new line, before the release is considered complete.
2. **Committed manifests use released versions only.** No `path` or `git`
   dependency on a sibling may be committed. Use a temporary, uncommitted
   `[patch.crates-io]` block to verify against unpublished sources, and
   regenerate `Cargo.lock` from the registry once the sibling is published.
3. **This matrix is regenerated on every sibling release** and its check runs
   in every sibling's release gate:

   ```sh
   scripts/check-stack-dependencies.py --check   # fails on lag, path/git pins, or a stale matrix
   scripts/check-stack-dependencies.py --write   # regenerate the block below
   ```

   The script reads each sibling checkout (`../grust`, `../typesec`,
   `../marciana`, `../lakecat`, `.`; override with `QG_STACK_<REPO>`), derives
   every cross-repository edge from the Cargo manifests, and compares each
   requirement with the sibling's current workspace version. Lag inside a
   `publish = false` crate is reported as a warning (it cannot break a
   registry consumer); `--strict` promotes it to a failure.
4. **Unpublished crates are still tracked.** A `publish = false` crate that
   pins an old sibling line is listed as *lagging (unpublished crate)* so the
   owner sees it before the crate is ever published.

## Current release line

| Repository | Version | Codename | Tag | Notes |
|---|---|---|---|---|
| grust | 0.13.0 | Prawn | `v0.13.0` | First lockstep line for all 15 publishable crates; Turso stable 0.7.2. |
| typesec | 0.14.0 | Dorsoduro | `v0.14.0` | Tracks Grust 0.13 from crates.io only (sibling `path` deps removed). |
| marciana | 0.13.1 | — | `v0.13.1` | Tracks Grust 0.13, TypeSec 0.14, LakeCat 0.4. |
| lakecat | 0.4.0 | Caracal | `v0.4.0` | Tracks Grust 0.13, TypeSec 0.14, Turso 0.7.2; Sail from `querygraph/sail#lakecat`. |
| querygraph | 0.5.0 | Harrier | `v0.5.0` | Tracks all of the above; Sail pin `c5309365`. |

Sail is consumed by LakeCat as a Cargo `git` dependency on the
`querygraph/sail` `lakecat` branch and by QueryGraph/Marciana as a pinned
server revision (`compat/sail-revision.txt`); it is not a crates.io line yet
and is tracked in `COMPATIBILITY.md`.

## Derived matrix

The block below is generated; do not edit it by hand.

<!-- stack-dependency-matrix:begin -->
| Repository | Workspace version | HEAD | Depends on |
|---|---|---|---|
| grust | `0.13.0` | `b6446dbc` | typesec |
| typesec | `0.14.0` | `3f12e622` | grust |
| marciana | `0.13.1` | `94e0b517` | grust, lakecat, typesec |
| lakecat | `0.4.0` | `ab07fb28` | grust, typesec |
| querygraph | `0.5.0` | `e9bdcef8` | grust, lakecat, marciana, typesec |

| Consumer | Manifest | Crate | Owner | Required | Owner version | Status |
|---|---|---|---|---|---|---|
| grust | `crates/querygraph-memory/Cargo.toml` | `typesec-core` | typesec | `0.13.0` | `0.14.0` | lagging (unpublished crate) |
| grust | `crates/querygraph-memory/Cargo.toml` | `typesec-memory` | typesec | `0.13.0` | `0.14.0` | lagging (unpublished crate) |
| grust | `crates/querygraph-memory/Cargo.toml` | `typesec-rbac` | typesec | `0.13.0` | `0.14.0` | lagging (unpublished crate) |
| typesec | `Cargo.toml` | `grust-cypher` | grust | `0.13.0` | `0.13.0` | aligned |
| typesec | `Cargo.toml` | `grust-graph` | grust | `0.13.0` | `0.13.0` | aligned |
| typesec | `Cargo.toml` | `grust-sail` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-catalog/Cargo.toml` | `lakecat-core` | lakecat | `0.4.0` | `0.4.0` | aligned |
| marciana | `crates/marciana-cognition/Cargo.toml` | `grust-graph` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-cognition/Cargo.toml` | `lakecat-core` | lakecat | `0.4.0` | `0.4.0` | aligned |
| marciana | `crates/marciana-cognition/Cargo.toml` | `typesec-core` | typesec | `0.14.0` | `0.14.0` | aligned |
| marciana | `crates/marciana-cognition/Cargo.toml` | `typesec-integrations` | typesec | `0.14.0` | `0.14.0` | aligned |
| marciana | `crates/marciana-cognition/Cargo.toml` | `typesec-memory` | typesec | `0.14.0` | `0.14.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `grust-core` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `grust-memory` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `grust-sail` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `grust-turso` | grust | `0.13.0` | `0.13.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `typesec-core` | typesec | `0.14.0` | `0.14.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `typesec-memory` | typesec | `0.14.0` | `0.14.0` | aligned |
| marciana | `crates/marciana-memory/Cargo.toml` | `typesec-rbac` | typesec | `0.14.0` | `0.14.0` | aligned |
| lakecat | `Cargo.toml` | `grust-graph` | grust | `0.13.0` | `0.13.0` | aligned |
| lakecat | `Cargo.toml` | `grust-turso` | grust | `0.13.0` | `0.13.0` | aligned |
| lakecat | `Cargo.toml` | `typesec` | typesec | `0.14.0` | `0.14.0` | aligned |
| querygraph | `Cargo.toml` | `grust-cypher` | grust | `0.13.0` | `0.13.0` | aligned |
| querygraph | `Cargo.toml` | `grust-graph` | grust | `0.13.0` | `0.13.0` | aligned |
| querygraph | `Cargo.toml` | `lakecat-core` | lakecat | `0.4.0` | `0.4.0` | aligned |
| querygraph | `Cargo.toml` | `marciana-cognition` | marciana | `0.13.1` | `0.13.1` | aligned |
| querygraph | `Cargo.toml` | `qglake-bundle` | lakecat | `0.4.0` | `0.4.0` | aligned |
| querygraph | `Cargo.toml` | `querygraph-memory` | marciana | `0.13.1` | `0.13.1` | aligned |
| querygraph | `Cargo.toml` | `typesec-agent` | typesec | `0.14.0` | `0.14.0` | aligned |
| querygraph | `Cargo.toml` | `typesec-core` | typesec | `0.14.0` | `0.14.0` | aligned |
| querygraph | `Cargo.toml` | `typesec-integrations` | typesec | `0.14.0` | `0.14.0` | aligned |
| querygraph | `Cargo.toml` | `typesec-memory` | typesec | `0.14.0` | `0.14.0` | aligned |
| querygraph | `Cargo.toml` | `typesec-rbac` | typesec | `0.14.0` | `0.14.0` | aligned |
<!-- stack-dependency-matrix:end -->
