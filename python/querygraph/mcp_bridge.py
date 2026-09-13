"""Compatibility entry point for the authoritative Rust MCP transport."""

from __future__ import annotations

import os
from pathlib import Path


async def serve_rust_bridge(executable: str | Path, *, registry_config: str | Path | None = None) -> None:
    """Replace this CLI process with Rust, preserving stdio and protocol IDs.

    The executable and registry configuration are deployment-owned paths.
    Rust owns initialization, cancellation, request bounds, and session cleanup.
    No second MCP session rewrites IDs or consumes cancellation notifications.
    """
    path = Path(executable).expanduser().resolve(strict=True)
    arguments = [str(path), "mcp-serve"]
    if registry_config is not None:
        arguments += ["--registry-config", str(Path(registry_config).expanduser().resolve(strict=True))]
    os.execv(path, arguments)
