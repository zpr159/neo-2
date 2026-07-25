"""Neo Neural Network Model — serializable model definition."""
from __future__ import annotations

import json
import uuid
from datetime import datetime, timezone
from pathlib import Path


class Model:
    """A serializable neural network model definition."""

    def __init__(self, name: str, config: dict | None = None) -> None:
        self.id: str = str(uuid.uuid4())
        self.name: str = name
        self.config: dict = config or {}
        self.parameters_count: int = 0
        self.state: str = "untrained"
        self.created_at: str = datetime.now(timezone.utc).isoformat()

    def save(self, path: str) -> None:
        """Serialize the model to a JSON file."""
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        Path(path).write_text(json.dumps(self.to_dict(), indent=2))

    @classmethod
    def load(cls, path: str) -> Model:
        """Load a model from a JSON file."""
        data = json.loads(Path(path).read_text())
        return cls.from_dict(data)

    def to_dict(self) -> dict:
        """Convert the model to a dictionary."""
        return {
            "id": self.id,
            "name": self.name,
            "config": self.config,
            "parameters_count": self.parameters_count,
            "state": self.state,
            "created_at": self.created_at,
        }

    @classmethod
    def from_dict(cls, data: dict) -> Model:
        """Create a Model from a dictionary."""
        model = cls(name=data["name"], config=data.get("config"))
        model.id = data.get("id", model.id)
        model.parameters_count = data.get("parameters_count", 0)
        model.state = data.get("state", "untrained")
        model.created_at = data.get("created_at", model.created_at)
        return model

    def __repr__(self) -> str:
        return f"Model(id={self.id!r}, name={self.name!r}, state={self.state!r})"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Model):
            return NotImplemented
        return self.to_dict() == other.to_dict()
