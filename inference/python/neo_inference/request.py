"""Neo Inference Request — typed inference request."""
from __future__ import annotations

import uuid


class InferenceRequest:
    """Represents a single inference request."""

    def __init__(self, model_id: str, input_data: dict, parameters: dict | None = None) -> None:
        self._id: str = str(uuid.uuid4())
        self._model_id: str = model_id
        self._input_data: dict = input_data
        self.parameters: dict = parameters or {}

    @property
    def id(self) -> str:
        return self._id

    @property
    def model_id(self) -> str:
        return self._model_id

    @property
    def input_data(self) -> dict:
        return self._input_data

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "model_id": self._model_id,
            "input_data": self._input_data,
            "parameters": self.parameters,
        }

    @classmethod
    def from_dict(cls, data: dict) -> InferenceRequest:
        req = cls(model_id=data["model_id"], input_data=data["input_data"], parameters=data.get("parameters"))
        req._id = data.get("id", req._id)
        return req
