#!/usr/bin/env python3
"""Pydantic AI v2 agents with TypeDID credentials and durable memory.

Runs without a model-provider key. The example builds and starts the root
QueryGraph service,
generates an exact-DID RBAC policy, writes a governed finding, restarts the
server, recalls it through another agent, and proves an outsider is denied.
"""

from __future__ import annotations

import asyncio
import json
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from querygraph.pydantic_ai_capabilities import (
    QueryGraphAgentDeps,
    run_recaller,
    run_specialist,
)
from querygraph.typedid import TypeDidAgent


PYTHON_ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = PYTHON_ROOT.parent


def available_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


@dataclass
class LocalQueryGraph:
    port: int
    policy: Path
    database: Path
    process: subprocess.Popen[bytes] | None = None

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def start(self) -> None:
        binary = RUST_ROOT / "target" / "debug" / "querygraph"
        self.process = subprocess.Popen(
            [
                str(binary),
                "serve",
                "--port",
                str(self.port),
                "--require-auth",
                "--memory-policy",
                str(self.policy),
                "--memory-db",
                str(self.database),
            ],
            cwd=RUST_ROOT,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        for _ in range(100):
            if self.process.poll() is not None:
                raise RuntimeError("QueryGraph exited before becoming healthy")
            try:
                with urllib.request.urlopen(f"{self.base_url}/v1/health", timeout=1):
                    return
            except OSError:
                time.sleep(0.1)
        raise RuntimeError("QueryGraph did not become healthy")

    def stop(self) -> None:
        if self.process is None:
            return
        self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5)
        self.process = None


def policy_for(*agents: TypeDidAgent) -> str:
    subjects = []
    for agent in agents:
        did = agent.did_key()
        if did is None:
            raise RuntimeError("install the crypto extra before running the demo")
        subjects.append(f'  - subject: "{did}"\n    roles: [research-memory]')
    return "\n".join(
        [
            "roles:",
            "  - name: research-memory",
            "    permissions: [read, write, delete]",
            '    resources: ["memory/team:marciana/shared"]',
            "assignments:",
            *subjects,
            "",
        ]
    )


def unsigned_probe(base_url: str) -> dict[str, Any]:
    request = urllib.request.Request(
        f"{base_url}/v1/answer",
        data=b'{"question":"show me protected data"}',
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        urllib.request.urlopen(request, timeout=5)
    except urllib.error.HTTPError as error:
        return {"status": error.code, "body": json.loads(error.read())}
    raise AssertionError("unsigned governed access unexpectedly succeeded")


async def run_demo() -> dict[str, Any]:
    specialist_credential = TypeDidAgent.new(
        "ResilienceSpecialist", seed="demo:credential:resilience-specialist"
    )
    supervisor_credential = TypeDidAgent.new(
        "ResearchSupervisor", seed="demo:credential:research-supervisor"
    )
    outsider_credential = TypeDidAgent.new(
        "UnassignedOutsider", seed="demo:credential:unassigned-outsider"
    )

    subprocess.run(["cargo", "build", "--quiet"], cwd=RUST_ROOT, check=True)
    with tempfile.TemporaryDirectory(prefix="querygraph-pydantic-v2-") as temp:
        root = Path(temp)
        policy = root / "memory-policy.yaml"
        database = root / "marciana.db"
        policy.write_text(
            policy_for(specialist_credential, supervisor_credential),
            encoding="utf-8",
        )
        server = LocalQueryGraph(available_port(), policy, database)

        try:
            server.start()
            unsigned = unsigned_probe(server.base_url)
            specialist = await run_specialist(
                QueryGraphAgentDeps(server.base_url, specialist_credential)
            )
        finally:
            server.stop()

        # The model registry was in-memory and is now gone. The Marciana graph
        # is file-backed, so a different credential can recover the finding.
        try:
            server.start()
            supervisor = await run_recaller(
                "research-supervisor",
                QueryGraphAgentDeps(server.base_url, supervisor_credential),
            )
            outsider = await run_recaller(
                "unassigned-outsider",
                QueryGraphAgentDeps(server.base_url, outsider_credential),
            )
        finally:
            server.stop()

        report = {
            "framework": "Pydantic AI v2",
            "model": "TestModel (deterministic; no provider key)",
            "stack": ["TypeDID", "TypeSec", "querygraph-memory", "Grust", "Turso"],
            "unsignedAccess": unsigned,
            "specialistBeforeRestart": specialist,
            "serverRestarted": True,
            "supervisorAfterRestart": supervisor,
            "outsiderAttempt": outsider,
        }
        assert unsigned["status"] == 401
        assert specialist["memoryId"]
        assert supervisor["hits"]
        assert outsider["denial"]
        return report


def main() -> None:
    report = asyncio.run(run_demo())
    print("\n╭──────────────────────────────────────────────────────────────╮")
    print("│  QueryGraph × Pydantic AI v2: credentials that remember     │")
    print("╰──────────────────────────────────────────────────────────────╯")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
