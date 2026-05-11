"""Tests for Browser Hub tab registry and attribution."""
import pytest
import time

from browser_hub import TabRegistry, TabEntry


class TestTabRegistry:
    def test_register_tab_agent(self):
        reg = TabRegistry()
        entry = reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert entry.owner == "agent:navigator"
        assert entry.tab_id == "tab-1"

    def test_register_tab_human(self):
        reg = TabRegistry()
        entry = reg.register("tab-2", "target-2", "human", "win-human")
        assert entry.owner == "human"

    def test_get_owner(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert reg.get_owner("tab-1") == "agent:navigator"

    def test_get_owner_unknown_tab(self):
        reg = TabRegistry()
        assert reg.get_owner("nonexistent") == "human"  # K14: unknown = human (safe default)

    def test_remove_tab(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        reg.remove("tab-1")
        assert reg.get_owner("tab-1") == "human"

    def test_list_tabs(self):
        reg = TabRegistry()
        reg.register("tab-1", "t-1", "agent:navigator", "win-agent")
        reg.register("tab-2", "t-2", "human", "win-human")
        tabs = reg.list_all()
        assert len(tabs) == 2

    def test_resolve_by_target_id(self):
        reg = TabRegistry()
        reg.register("tab-1", "target-1", "agent:navigator", "win-agent")
        assert reg.resolve_target("target-1") == "agent:navigator"

    def test_resolve_unknown_target(self):
        reg = TabRegistry()
        assert reg.resolve_target("unknown") == "human"


class TestAttributionBuffer:
    def test_record_and_match(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=100)
        now = time.time()
        buf.record("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now, "agent:navigator")
        result = buf.match("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now + 0.05)
        assert result == "agent:navigator"

    def test_match_expired(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=1, max_size=100)
        now = time.time()
        buf.record("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now, "agent:navigator")
        result = buf.match("https://x.com/i/api/graphql/abc/SearchTimeline?v=1", now + 2.0)
        assert result == "unknown"

    def test_match_no_entry(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=100)
        result = buf.match("https://x.com/something", time.time())
        assert result == "unknown"

    def test_buffer_eviction(self):
        from browser_hub import AttributionBuffer
        buf = AttributionBuffer(ttl_seconds=5, max_size=3)
        now = time.time()
        buf.record("url1", now, "a")
        buf.record("url2", now, "b")
        buf.record("url3", now, "c")
        buf.record("url4", now, "d")  # should evict oldest
        assert buf.match("url1", now) == "unknown"  # evicted
        assert buf.match("url4", now) == "d"


import aiohttp
from aiohttp.test_utils import AioHTTPTestCase, unittest_run_loop

from browser_hub import create_app, AttributionBuffer


class TestHubHTTPAPI(AioHTTPTestCase):
    async def get_application(self):
        registry = TabRegistry()
        attribution = AttributionBuffer()
        registry.register("tab-1", "target-1", "agent:navigator", "win-agent", url="https://x.com/search")
        registry.register("tab-2", "target-2", "human", "win-human", url="https://x.com/home")
        return create_app(registry, attribution)

    @unittest_run_loop
    async def test_get_status(self):
        resp = await self.client.request("GET", "/status")
        assert resp.status == 200
        data = await resp.json()
        assert "tabs" in data

    @unittest_run_loop
    async def test_get_tabs(self):
        resp = await self.client.request("GET", "/tabs")
        assert resp.status == 200
        data = await resp.json()
        assert len(data) == 2

    @unittest_run_loop
    async def test_get_tab_owner(self):
        resp = await self.client.request("GET", "/tabs/tab-1/owner")
        assert resp.status == 200
        data = await resp.json()
        assert data["owner"] == "agent:navigator"

    @unittest_run_loop
    async def test_get_tab_owner_unknown(self):
        resp = await self.client.request("GET", "/tabs/nonexistent/owner")
        assert resp.status == 200
        data = await resp.json()
        assert data["owner"] == "human"

    @unittest_run_loop
    async def test_get_attribution_unknown(self):
        resp = await self.client.request("GET", "/attribution?url=https://x.com/something&ts=0")
        assert resp.status == 200
        data = await resp.json()
        assert data["source"] == "unknown"

    @unittest_run_loop
    async def test_post_create_tab(self):
        resp = await self.client.request("POST", "/tabs", json={
            "owner": "agent:search-executor",
            "url": "https://x.com/search?q=test",
            "window": "agent",
        })
        assert resp.status == 201
        data = await resp.json()
        assert "tab_id" in data

    @unittest_run_loop
    async def test_delete_tab(self):
        resp = await self.client.request("DELETE", "/tabs/tab-1")
        assert resp.status == 200
        resp2 = await self.client.request("GET", "/tabs/tab-1/owner")
        data = await resp2.json()
        assert data["owner"] == "human"
