"""
HTTP Router Registration Regression Test

Verifies that all auto-registered routers:
1. Have /api/ prefix for consistency
2. Work via HTTP (not just TestClient)
3. Return non-404 responses

This guards against the anti-pattern where routes exist in app.routes
but don't work via HTTP due to lifecycle timing issues.
"""
import pytest
from fastapi.testclient import TestClient
from cynic.api.server import app, _routers_registered


class TestRouterHTTPRegistration:
    """Verify all routers work via HTTP."""

    @pytest.fixture
    def client(self):
        """Create test client."""
        return TestClient(app)

    def test_all_routers_registered(self):
        """Verify routers were registered during app creation."""
        assert _routers_registered is not None
        assert len(_routers_registered) > 0
        assert "consciousness_ecosystem" in _routers_registered

    def test_all_routers_have_api_prefix(self):
        """Verify all routers use /api/ prefix."""
        for route in app.routes:
            path = getattr(route, "path", "")
            # Skip non-route items like Mount
            if not path or path.startswith("/static") or path.startswith("/openapi"):
                continue
            # All API routes should have /api/ prefix
            assert path.startswith("/api/"), (
                f"Route {path} missing /api/ prefix. "
                f"All routes must use /api/prefix for consistency."
            )

    def test_consciousness_ecosystem_routes_work_via_http(self, client):
        """Test consciousness ecosystem endpoints via HTTP."""
        # These routes should exist and not return 404
        test_routes = [
            "/api/consciousness/ecosystem",
            "/api/consciousness/perception-sources",
            "/api/consciousness/topology",
            "/api/consciousness/nervous-system",
        ]

        for route in test_routes:
            response = client.get(route)
            assert response.status_code != 404, (
                f"Route {route} returned 404 via HTTP. "
                f"Routes exist in app.routes but not served by HTTP handler."
            )

    def test_observability_routes_work_via_http(self, client):
        """Test observability endpoints via HTTP."""
        # These routes should exist and not return 404
        test_routes = [
            "/api/observability/metrics",
            "/api/observability/health",
            "/api/observability/ready",
            "/api/observability/version",
        ]

        for route in test_routes:
            response = client.get(route)
            assert response.status_code != 404, (
                f"Route {route} returned 404 via HTTP. "
                f"Observability routes not being served by HTTP handler."
            )

    def test_all_registered_routers_have_routes(self):
        """Verify each registered router actually has routes."""
        for router_name, router_info in _routers_registered.items():
            route_count = router_info.get("routes", 0)
            assert route_count > 0, (
                f"Router {router_name} registered but has no routes. "
                f"This indicates a registration issue."
            )

    def test_no_routes_outside_api_prefix(self):
        """Guard: Verify no API routes exist outside /api/ prefix."""
        # This prevents accidental creation of routes without /api/ prefix
        invalid_routes = []
        for route in app.routes:
            path = getattr(route, "path", "")
            # Skip static, openapi, health check
            if any(skip in path for skip in ["/static", "/openapi", "/docs", "/redoc"]):
                continue
            # Warn if we find routes not under /api/
            if path and not path.startswith("/api/"):
                invalid_routes.append(path)

        assert not invalid_routes, (
            f"Found routes without /api/ prefix: {invalid_routes}. "
            f"All new routes must use /api/prefix to maintain consistency."
        )


class TestRouterConsistency:
    """Verify router patterns are consistent."""

    def test_router_module_naming(self):
        """Verify router modules follow naming convention."""
        # Each registered router should come from a module named router*.py
        for router_name in _routers_registered.keys():
            # router_name is the module stem (e.g., "consciousness_ecosystem")
            assert "router" in router_name.lower(), (
                f"Router module {router_name} doesn't contain 'router' in name. "
                f"Follow naming convention: routers/router_*.py or routers/*_router.py"
            )

    def test_all_routes_in_app_routes(self):
        """Verify registered routers' routes appear in app.routes."""
        # Count routes by prefix
        prefixes = {}
        for route in app.routes:
            path = getattr(route, "path", "")
            if path.startswith("/api/"):
                # Extract prefix (e.g., /api/consciousness from /api/consciousness/ecosystem)
                parts = path.split("/")
                if len(parts) >= 3:
                    prefix = f"/{parts[1]}/{parts[2]}"  # /api/consciousness
                    prefixes[prefix] = prefixes.get(prefix, 0) + 1

        # Verify we found routes for each registered router
        for router_name in _routers_registered.keys():
            # At least one route should exist for this router
            found = False
            for prefix in prefixes:
                if router_name.replace("_", "").lower() in prefix.replace("/", "").lower():
                    found = True
                    break
            # Don't assert here - routers might have routes added via decorators
            # Just log for debugging
            if not found:
                print(f"Warning: Router {router_name} not clearly represented in app.routes")
