"""Tests for neo_tools."""
from neo_tools.tool import Tool, ToolCategory, ToolStatus
from neo_tools.registry import ToolRegistry


class TestTool:
    def test_create_tool(self):
        t = Tool("web_search", "Search the web", ToolCategory.Search)
        assert t.name == "web_search"
        assert t.status == ToolStatus.Enabled

    def test_enable_disable(self):
        t = Tool("t", "d", ToolCategory.Utility)
        t.disable()
        assert t.status == ToolStatus.Disabled
        t.enable()
        assert t.status == ToolStatus.Enabled


class TestToolRegistry:
    def test_register_and_get(self):
        r = ToolRegistry()
        t = Tool("t1", "desc", ToolCategory.Code)
        tid = r.register(t)
        assert r.get(tid) is t

    def test_unregister(self):
        r = ToolRegistry()
        t = Tool("t", "d", ToolCategory.Data)
        tid = r.register(t)
        assert r.unregister(tid) is True
        assert r.get(tid) is None

    def test_list_all(self):
        r = ToolRegistry()
        r.register(Tool("a", "ad", ToolCategory.Search))
        r.register(Tool("b", "bd", ToolCategory.Code))
        assert r.count() == 2

    def test_search(self):
        r = ToolRegistry()
        r.register(Tool("web_search", "Search the internet", ToolCategory.Search))
        r.register(Tool("code_gen", "Generate code", ToolCategory.Code))
        results = r.search("search")
        assert len(results) == 1
        assert results[0].name == "web_search"

    def test_count(self):
        r = ToolRegistry()
        assert r.count() == 0
        r.register(Tool("x", "x", ToolCategory.Utility))
        assert r.count() == 1
