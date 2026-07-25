"""Neo Inference Response — typed inference response."""
from __future__ import annotations


class InferenceResponse:
    """Represents the result of an inference request."""

    def __init__(
        self,
        request_id: str,
        status: str,
        output: dict | None = None,
        latency_ms: float = 0.0,
        error: str | None = None,
    ) -> None:
        self.request_id = request_id
        self.status = status
        self.output = output
        self.latency_ms = latency_ms
        self.error = error

    @classmethod
    def success(cls, request_id: str, output: dict, latency_ms: float = 0.0) -> InferenceResponse:
        return cls(request_id=request_id, status="success", output=output, latency_ms=latency_ms)

    @classmethod
    def error(cls, request_id: str, error: str, latency_ms: float = 0.0) -> InferenceResponse:
        return cls(request_id=request_id, status="error", error=error, latency_ms=latency_ms)

    def to_dict(self) -> dict:
        return {
            "request_id": self.request_id,
            "status": self.status,
            "output": self.output,
            "latency_ms": self.latency_ms,
            "error": self.error,
        }
