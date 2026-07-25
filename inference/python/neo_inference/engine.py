"""Neo Inference Engine — model loading and inference execution."""
from __future__ import annotations

from neo_inference.request import InferenceRequest
from neo_inference.response import InferenceResponse


class InferenceEngine:
    """Manages loaded models and runs inference requests."""

    def __init__(self, config: dict | None = None) -> None:
        self.config = config or {}
        self._models: dict[str, dict] = {}

    def load_model(self, model_id: str, path: str) -> None:
        """Load a model into memory."""
        self._models[model_id] = {"path": path, "loaded": True}

    def infer(self, request: InferenceRequest) -> InferenceResponse:
        """Run inference on a request."""
        if request.model_id not in self._models:
            return InferenceResponse.error(
                request_id=request.id,
                error=f"Model {request.model_id} not loaded",
                latency_ms=0.0,
            )
        return InferenceResponse.success(
            request_id=request.id,
            output={"result": "stub"},
            latency_ms=0.0,
        )

    def loaded_models(self) -> list[str]:
        """Return IDs of all loaded models."""
        return list(self._models.keys())

    def unload_model(self, model_id: str) -> None:
        """Unload a model from memory."""
        self._models.pop(model_id, None)
