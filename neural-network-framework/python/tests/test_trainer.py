"""Tests for neo_nn.trainer."""
import os
import tempfile

from neo_nn.model import Model
from neo_nn.trainer import Trainer


class TestTrainerCreation:
    def test_create_trainer(self):
        m = Model(name="m")
        t = Trainer(m)
        assert t.model is m
        assert t.config == {}

    def test_create_with_config(self):
        m = Model(name="m")
        t = Trainer(m, config={"lr": 0.001})
        assert t.config == {"lr": 0.001}


class TestTrainerOperations:
    def test_train_step(self):
        t = Trainer(Model(name="m"))
        result = t.train_step({"x": [1, 2, 3]})
        assert "loss" in result
        assert "step" in result
        assert result["step"] == 1

    def test_train_step_increments(self):
        t = Trainer(Model(name="m"))
        t.train_step({})
        r2 = t.train_step({})
        assert r2["step"] == 2

    def test_evaluate(self):
        t = Trainer(Model(name="m"))
        result = t.evaluate([{"x": 1}, {"x": 2}])
        assert "loss" in result
        assert "accuracy" in result

    def test_checkpoint(self):
        t = Trainer(Model(name="m"))
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            t.checkpoint(path)
            assert os.path.exists(path)
        finally:
            os.unlink(path)

    def test_state(self):
        t = Trainer(Model(name="m"))
        s = t.state
        assert "model_id" in s
        assert "step" in s
