"""Neo Reasoning — chain-of-thought reasoning."""
from __future__ import annotations

import uuid
from enum import Enum


class ReasoningStepType(Enum):
    Premise = "premise"
    Inference = "inference"
    Conclusion = "conclusion"
    Observation = "observation"
    Hypothesis = "hypothesis"
    Evaluation = "evaluation"


class ReasoningStep:
    """A single step in a reasoning chain."""

    def __init__(
        self,
        step_type: ReasoningStepType,
        content: str,
        confidence: float = 1.0,
        source: str | None = None,
    ) -> None:
        self._id: str = str(uuid.uuid4())
        self.step_type = step_type
        self.content = content
        self.confidence = confidence
        self.source = source

    @property
    def id(self) -> str:
        return self._id

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "step_type": self.step_type.value,
            "content": self.content,
            "confidence": self.confidence,
            "source": self.source,
        }


class ReasoningChain:
    """An ordered chain of reasoning steps."""

    def __init__(self, strategy: str = "chain_of_thought") -> None:
        self.strategy = strategy
        self._steps: list[ReasoningStep] = []

    def add_step(self, step: ReasoningStep) -> None:
        self._steps.append(step)

    def confidence(self) -> float:
        if not self._steps:
            return 0.0
        total = 1.0
        for step in self._steps:
            total *= step.confidence
        return total

    def is_valid(self) -> bool:
        if not self._steps:
            return False
        has_conclusion = any(s.step_type == ReasoningStepType.Conclusion for s in self._steps)
        has_premise = any(s.step_type == ReasoningStepType.Premise for s in self._steps)
        return has_premise and has_conclusion

    def conclusion(self) -> str | None:
        for step in reversed(self._steps):
            if step.step_type == ReasoningStepType.Conclusion:
                return step.content
        return None

    def to_dict(self) -> dict:
        return {
            "strategy": self.strategy,
            "steps": [s.to_dict() for s in self._steps],
            "confidence": self.confidence(),
        }
