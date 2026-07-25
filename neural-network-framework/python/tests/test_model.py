"""Tests for neo_nn.model."""
import json
import os
import tempfile

from neo_nn.model import Model


class TestModelCreation:
    def test_create_model(self):
        m = Model(name="test")
        assert m.name == "test"
        assert m.state == "untrained"
        assert m.parameters_count == 0
        assert isinstance(m.id, str)
        assert len(m.id) == 36

    def test_create_with_config(self):
        m = Model(name="m", config={"lr": 0.01})
        assert m.config == {"lr": 0.01}

    def test_repr(self):
        m = Model(name="x")
        assert "Model(" in repr(m)
        assert "x" in repr(m)


class TestModelPersistence:
    def test_save_load_roundtrip(self):
        m = Model(name="persist", config={"batch": 32})
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
            path = f.name
        try:
            m.save(path)
            loaded = Model.load(path)
            assert loaded.name == m.name
            assert loaded.config == m.config
            assert loaded.id == m.id
        finally:
            os.unlink(path)

    def test_to_dict_from_dict_roundtrip(self):
        m = Model(name="round", config={"k": "v"})
        d = m.to_dict()
        restored = Model.from_dict(d)
        assert m == restored


class TestModelEquality:
    def test_equal_models(self):
        d = {"id": "abc", "name": "n", "config": {}, "parameters_count": 0, "state": "untrained", "created_at": "2025-01-01T00:00:00"}
        a = Model.from_dict(d)
        b = Model.from_dict(d)
        assert a == b

    def test_different_models(self):
        a = Model(name="a")
        b = Model(name="b")
        assert a != b
