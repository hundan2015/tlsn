from __future__ import annotations

from typing import Any

from . import tlsn_python as _native
from .models import HttpRequest, ProverConfig, Reveal, VerifierConfig


async def initialize(logging_config: Any | None = None, thread_count: int = 1) -> None:
    await _native.initialize(logging_config, thread_count)


def compute_reveal(sent: bytes, recv: bytes, handlers: list[dict[str, Any]]) -> dict[str, Any]:
    return _native.compute_reveal(sent, recv, handlers)


class Prover:
    def __init__(self, config: ProverConfig | dict[str, Any]) -> None:
        wire = config.to_wire() if isinstance(config, ProverConfig) else config
        self._inner = _native.Prover(wire)

    def set_progress_callback(self, callback: Any | None = None) -> None:
        self._inner.set_progress_callback(callback)

    async def setup(self, verifier_addr: str) -> None:
        await self._inner.setup(verifier_addr)

    async def send_request(
        self,
        server_addr: str,
        request: HttpRequest | dict[str, Any],
    ) -> dict[str, Any]:
        wire = request.to_wire() if isinstance(request, HttpRequest) else request
        return await self._inner.send_request(server_addr, wire)

    def transcript(self) -> dict[str, Any]:
        return self._inner.transcript()

    async def reveal(self, reveal: Reveal | dict[str, Any]) -> None:
        wire = reveal.to_wire() if isinstance(reveal, Reveal) else reveal
        await self._inner.reveal(wire)


class Verifier:
    def __init__(self, config: VerifierConfig | dict[str, Any]) -> None:
        wire = config.to_wire() if isinstance(config, VerifierConfig) else config
        self._inner = _native.Verifier(wire)

    async def connect(self, prover_addr: str) -> None:
        await self._inner.connect(prover_addr)

    async def verify(self) -> dict[str, Any]:
        return await self._inner.verify()
