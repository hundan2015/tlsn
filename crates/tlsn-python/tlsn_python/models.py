from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class Method(str, Enum):
    GET = "GET"
    POST = "POST"
    PUT = "PUT"
    DELETE = "DELETE"


class NetworkSetting(str, Enum):
    Bandwidth = "Bandwidth"
    Latency = "Latency"


@dataclass(slots=True)
class ProverConfig:
    server_name: str
    max_sent_data: int
    max_recv_data: int
    network: NetworkSetting = NetworkSetting.Latency
    max_sent_records: int | None = None
    max_recv_data_online: int | None = None
    max_recv_records_online: int | None = None
    defer_decryption_from_start: bool | None = None
    client_auth: tuple[list[bytes], bytes] | None = None

    def to_wire(self) -> dict[str, Any]:
        return {
            "server_name": self.server_name,
            "max_sent_data": self.max_sent_data,
            "max_sent_records": self.max_sent_records,
            "max_recv_data_online": self.max_recv_data_online,
            "max_recv_data": self.max_recv_data,
            "max_recv_records_online": self.max_recv_records_online,
            "defer_decryption_from_start": self.defer_decryption_from_start,
            "network": self.network.value,
            "client_auth": self.client_auth,
        }


@dataclass(slots=True)
class VerifierConfig:
    max_sent_data: int
    max_recv_data: int
    max_sent_records: int | None = None
    max_recv_records_online: int | None = None

    def to_wire(self) -> dict[str, Any]:
        return {
            "max_sent_data": self.max_sent_data,
            "max_recv_data": self.max_recv_data,
            "max_sent_records": self.max_sent_records,
            "max_recv_records_online": self.max_recv_records_online,
        }


@dataclass(slots=True)
class HttpRequest:
    uri: str
    method: Method = Method.GET
    headers: dict[str, bytes] = field(default_factory=dict)
    body: Any | None = None

    def to_wire(self) -> dict[str, Any]:
        body = None
        if self.body is not None:
            body = self.body

        return {
            "uri": self.uri,
            "method": self.method.value,
            "headers": self.headers,
            "body": body,
        }


@dataclass(slots=True)
class Reveal:
    sent: list[tuple[int, int]] = field(default_factory=list)
    recv: list[tuple[int, int]] = field(default_factory=list)
    server_identity: bool = False

    def to_wire(self) -> dict[str, Any]:
        return {
            "sent": [{"start": s, "end": e} for s, e in self.sent],
            "recv": [{"start": s, "end": e} for s, e in self.recv],
            "server_identity": self.server_identity,
        }
