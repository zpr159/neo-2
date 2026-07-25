"""Neo Neural Network Trainer — training loop and checkpointing."""
from __future__ import annotations

import json
from pathlib import Path

from neo_nn.model import Model


class Trainer:
    """Handles training, evaluation, and checkpointing for a Model."""

    def __init__(self, model: Model, config: dict | None = None) -> None:
        self.model = model
        self.config = config or {}
        self._step: int = 0

    def train_step(self, batch: dict) -> dict:
        """Execute a single training step on a batch.

        Returns:
            Dictionary with 'loss' and 'step' keys.
        """
        self._step += 1
        return {"loss": 0.0, "step": self._step}

    def evaluate(self, dataset: list[dict]) -> dict:
        """Evaluate the model on a dataset.

        Returns:
            Dictionary with 'loss' and 'accuracy' keys.
        """
        return {"loss": 0.0, "accuracy": 0.0}

    def checkpoint(self, path: str) -> None:
        """Save a training checkpoint to disk."""
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        checkpoint_data = {
            "model": self.model.to_dict(),
            "config": self.config,
            "step": self._step,
        }
        Path(path).write_text(json.dumps(checkpoint_data, indent=2))

    @property
    def state(self) -> dict:
        """Return the current trainer state."""
        return {
            "model_id": self.model.id,
            "step": self._step,
            "config": self.config,
        }
