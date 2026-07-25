"""Tests for neo_plugins."""
from neo_plugins.plugin import Plugin, PluginMetadata, PluginState


class TestPluginMetadata:
    def test_create_metadata(self):
        m = PluginMetadata("my-plugin", "1.0.0", "author", "desc")
        assert m.name == "my-plugin"
        assert m.version == "1.0.0"


class TestPlugin:
    def test_create_plugin(self):
        m = PluginMetadata("p", "0.1.0")
        p = Plugin(m)
        assert p.state == PluginState.Registered

    def test_load_activate_deactivate(self):
        m = PluginMetadata("p", "0.1.0")
        p = Plugin(m)
        p.load()
        assert p.state == PluginState.Loaded
        p.activate()
        assert p.state == PluginState.Active
        p.deactivate()
        assert p.state == PluginState.Loaded

    def test_error_on_invalid_transition(self):
        m = PluginMetadata("p", "0.1.0")
        p = Plugin(m)
        p.activate()  # can't activate from registered
        assert p.state == PluginState.Error

    def test_to_dict(self):
        m = PluginMetadata("p", "0.1.0", "me", "d")
        p = Plugin(m)
        d = p.to_dict()
        assert d["name"] == "p"
        assert d["version"] == "0.1.0"
