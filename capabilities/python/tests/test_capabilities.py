"""Tests for neo_capabilities."""
from neo_capabilities.capability import Capability, CapabilityType, CapabilityState
from neo_capabilities.registry import CapabilityRegistry


class TestCapability:
    def test_create(self):
        c = Capability("search", CapabilityType.Tool, "Web search")
        assert c.name == "search"
        assert c.state == CapabilityState.Registered

    def test_enable_disable(self):
        c = Capability("x", CapabilityType.Skill)
        c.enable()
        assert c.state == CapabilityState.Active
        c.disable()
        assert c.state == CapabilityState.Disabled

    def test_revoke(self):
        c = Capability("x", CapabilityType.Skill)
        c.revoke()
        assert c.state == CapabilityState.Revoked

    def test_to_dict(self):
        c = Capability("y", CapabilityType.Knowledge, "stuff")
        d = c.to_dict()
        assert d["name"] == "y"
        assert d["cap_type"] == "knowledge"


class TestCapabilityRegistry:
    def test_register_and_get(self):
        r = CapabilityRegistry()
        c = Capability("tool1", CapabilityType.Tool)
        cid = r.register(c)
        assert r.get(cid) is c

    def test_unregister(self):
        r = CapabilityRegistry()
        c = Capability("t", CapabilityType.Tool)
        cid = r.register(c)
        assert r.unregister(cid) is True
        assert r.get(cid) is None

    def test_unregister_missing(self):
        r = CapabilityRegistry()
        assert r.unregister("nope") is False

    def test_list_all(self):
        r = CapabilityRegistry()
        r.register(Capability("a", CapabilityType.Tool))
        r.register(Capability("b", CapabilityType.Skill))
        assert len(r.list_all()) == 2

    def test_has(self):
        r = CapabilityRegistry()
        r.register(Capability("found", CapabilityType.Tool))
        assert r.has("found") is True
        assert r.has("missing") is False
