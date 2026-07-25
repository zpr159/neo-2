"""Tests for neo_inference."""
from neo_inference.engine import InferenceEngine
from neo_inference.request import InferenceRequest
from neo_inference.response import InferenceResponse


class TestInferenceEngine:
    def test_create_engine(self):
        e = InferenceEngine()
        assert e.loaded_models() == []

    def test_load_model(self):
        e = InferenceEngine()
        e.load_model("m1", "/tmp/model.json")
        assert "m1" in e.loaded_models()

    def test_unload_model(self):
        e = InferenceEngine()
        e.load_model("m1", "/tmp/model.json")
        e.unload_model("m1")
        assert "m1" not in e.loaded_models()

    def test_infer_unloaded_model(self):
        e = InferenceEngine()
        req = InferenceRequest(model_id="missing", input_data={"x": 1})
        resp = e.infer(req)
        assert resp.status == "error"

    def test_infer_loaded_model(self):
        e = InferenceEngine()
        e.load_model("m1", "/tmp/model.json")
        req = InferenceRequest(model_id="m1", input_data={"x": 1})
        resp = e.infer(req)
        assert resp.status == "success"


class TestInferenceRequest:
    def test_create_request(self):
        r = InferenceRequest(model_id="m", input_data={"a": 1})
        assert r.model_id == "m"
        assert len(r.id) == 36

    def test_to_dict_from_dict(self):
        r = InferenceRequest(model_id="m", input_data={"a": 1})
        d = r.to_dict()
        r2 = InferenceRequest.from_dict(d)
        assert r.id == r2.id
        assert r.model_id == r2.model_id


class TestInferenceResponse:
    def test_success_response(self):
        r = InferenceResponse.success("req1", {"out": 1}, 1.5)
        assert r.status == "success"
        assert r.latency_ms == 1.5

    def test_error_response(self):
        r = InferenceResponse.error("req1", "bad input")
        assert r.status == "error"
        assert r.error == "bad input"
