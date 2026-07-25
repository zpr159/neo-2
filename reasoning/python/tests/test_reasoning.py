"""Tests for neo_reasoning."""
from neo_reasoning.chain import ReasoningChain, ReasoningStep, ReasoningStepType


class TestReasoningStep:
    def test_create_step(self):
        s = ReasoningStep(ReasoningStepType.Premise, "All men are mortal")
        assert s.step_type == ReasoningStepType.Premise
        assert s.content == "All men are mortal"
        assert s.confidence == 1.0

    def test_step_to_dict(self):
        s = ReasoningStep(ReasoningStepType.Inference, "Socrates is a man", 0.9)
        d = s.to_dict()
        assert d["step_type"] == "inference"
        assert d["confidence"] == 0.9


class TestReasoningChain:
    def test_create_chain(self):
        c = ReasoningChain()
        assert c.strategy == "chain_of_thought"

    def test_add_step(self):
        c = ReasoningChain()
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P1"))
        c.add_step(ReasoningStep(ReasoningStepType.Conclusion, "C1"))
        assert len(c._steps) == 2

    def test_confidence(self):
        c = ReasoningChain()
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P", 0.9))
        c.add_step(ReasoningStep(ReasoningStepType.Conclusion, "C", 0.8))
        assert abs(c.confidence() - 0.72) < 1e-9

    def test_is_valid(self):
        c = ReasoningChain()
        assert c.is_valid() is False
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P"))
        assert c.is_valid() is False
        c.add_step(ReasoningStep(ReasoningStepType.Conclusion, "C"))
        assert c.is_valid() is True

    def test_conclusion(self):
        c = ReasoningChain()
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P"))
        c.add_step(ReasoningStep(ReasoningStepType.Conclusion, "The answer"))
        assert c.conclusion() == "The answer"

    def test_no_conclusion(self):
        c = ReasoningChain()
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P"))
        assert c.conclusion() is None

    def test_to_dict(self):
        c = ReasoningChain("deduction")
        c.add_step(ReasoningStep(ReasoningStepType.Premise, "P"))
        d = c.to_dict()
        assert d["strategy"] == "deduction"
        assert len(d["steps"]) == 1
