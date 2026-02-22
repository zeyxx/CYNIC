#!/usr/bin/env python3
"""
Export OpenAPI schema from FastAPI app and generate Markdown documentation.

Usage:
  python scripts/export_openapi.py [output_dir]

Outputs:
  - openapi.json (raw OpenAPI schema)
  - docs/api-reference.md (human-readable API docs)
"""
import json
import sys
import os
from pathlib import Path

# Set working directory
os.chdir(Path(__file__).parent.parent)
sys.path.insert(0, str(Path(__file__).parent.parent))

from cynic.api.server import app


def generate_markdown_docs(openapi_spec: dict) -> str:
    """Convert OpenAPI spec to readable Markdown documentation."""

    title = openapi_spec.get("info", {}).get("title", "API Reference")
    description = openapi_spec.get("info", {}).get("description", "")
    version = openapi_spec.get("info", {}).get("version", "1.0.0")

    md = f"""# {title}

**Version:** {version}

{description}

## Overview

This document describes all available endpoints in the CYNIC Kernel API.

- **Base URL:** `http://localhost:8000` (local) or deployed URL
- **Response format:** JSON (all endpoints return JSON)
- **Authentication:** None (local development — add in production)

---

## Endpoints by Tag

"""

    paths = openapi_spec.get("paths", {})
    tags_map = {}

    # Group endpoints by tag
    for path, path_item in sorted(paths.items()):
        for method, operation in path_item.items():
            if method in ["get", "post", "put", "delete", "patch"]:
                tags = operation.get("tags", ["default"])
                for tag in tags:
                    if tag not in tags_map:
                        tags_map[tag] = []
                    tags_map[tag].append((method.upper(), path, operation))

    # Generate docs per tag
    for tag in sorted(tags_map.keys()):
        md += f"\n### {tag.capitalize()}\n\n"

        for method, path, operation in sorted(tags_map[tag]):
            summary = operation.get("summary", "")
            description_text = operation.get("description", "")

            # Endpoint header
            md += f"#### `{method} {path}`\n\n"

            if summary:
                md += f"**Summary:** {summary}\n\n"

            if description_text:
                md += f"**Description:**\n\n{description_text}\n\n"

            # Parameters
            parameters = operation.get("parameters", [])
            if parameters:
                md += "**Parameters:**\n\n"
                for param in parameters:
                    param_name = param.get("name", "unknown")
                    param_in = param.get("in", "query")
                    param_required = param.get("required", False)
                    param_schema = param.get("schema", {})
                    param_type = param_schema.get("type", "string")
                    param_desc = param.get("description", "")

                    required_text = " (required)" if param_required else " (optional)"
                    md += f"- `{param_name}` ({param_in}, {param_type}){required_text}: {param_desc}\n"
                md += "\n"

            # Request body
            request_body = operation.get("requestBody", {})
            if request_body:
                md += "**Request Body:**\n\n"
                md += "```json\n"
                content = request_body.get("content", {})
                for content_type, content_spec in content.items():
                    if "example" in content_spec:
                        md += json.dumps(content_spec["example"], indent=2)
                    elif "schema" in content_spec:
                        md += json.dumps(content_spec["schema"], indent=2)
                md += "\n```\n\n"

            # Response
            responses = operation.get("responses", {})
            if responses:
                md += "**Responses:**\n\n"
                for status_code, response_spec in sorted(responses.items()):
                    description_resp = response_spec.get("description", "")
                    md += f"- **{status_code}**: {description_resp}\n"
                md += "\n"

            # Example curl command
            md += f"**Example:**\n\n```bash\ncurl -X {method} http://localhost:8000{path}\n```\n\n"
            md += "---\n\n"

    # Add section for health checks and monitoring
    md += """
## Health Check

The API provides health check endpoint to verify server status:

- **Endpoint:** `GET /health` or `GET /api/health`
- **Response:** JSON with organism health status
- **Status codes:** 200 (healthy), 503 (degraded)

## Metrics

Prometheus metrics are available at:

- **Endpoint:** `GET /metrics`
- **Format:** Prometheus text format
- **Common metrics:**
  - `cynic_requests_total` — Total HTTP requests by endpoint, method, status
  - `cynic_request_duration_seconds` — Request duration distribution by endpoint
  - `cynic_learning_rate` — Current learning rate [0, 0.618]
  - `cynic_consciousness_level` — Current consciousness LOD [0, 3]

## WebSocket

Real-time streaming is available via WebSocket:

- **Endpoint:** `GET /ws/stream`
- **Message format:** JSON events from organism event bus
- **Useful for:** Live dashboard, monitoring, real-time feedback loops

## Error Handling

All errors return structured JSON with three fields:

```json
{
  "error": "User-friendly error message",
  "code": "ERROR_CODE",
  "type": "ErrorType"
}
```

Common error codes:
- `VALIDATION_ERROR` — Request validation failed
- `NOT_FOUND` — Resource not found
- `INTERNAL_ERROR` — Server error

## Rate Limiting

No rate limits on local instance. On production:
- Implement rate limiting per API key
- Max 100 req/s per endpoint
- Budget-aware throttling (if spend > budget)

## CORS

CORS is enabled for local development (all origins allowed). Restrict in production.

## Examples

### Get organism state snapshot

```bash
curl http://localhost:8000/api/organism/state/snapshot
```

### Get consciousness level

```bash
curl http://localhost:8000/api/organism/consciousness
```

### Get all dogs

```bash
curl http://localhost:8000/api/organism/dogs
```

### Get pending actions

```bash
curl http://localhost:8000/api/organism/actions
```

### Get account status

```bash
curl http://localhost:8000/api/organism/account
```

### Get learned policy actions

```bash
curl http://localhost:8000/api/organism/policy/actions
```

### Get policy statistics

```bash
curl http://localhost:8000/api/organism/policy/stats
```

### Watch live events (WebSocket)

```bash
wscat -c ws://localhost:8000/ws/stream
```

## Interactive Swagger UI

The API provides interactive Swagger UI:

- **URL:** `http://localhost:8000/docs`
- **Features:** Try endpoints, see schemas, auto-generated from code
- **Alternative:** ReDoc at `http://localhost:8000/redoc`

---

*Generated from FastAPI OpenAPI schema*
"""

    return md


def main():
    """Export API docs."""
    output_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("cynic/docs")
    output_dir.mkdir(parents=True, exist_ok=True)

    # Get OpenAPI spec - handle schema errors gracefully
    try:
        spec = app.openapi()
    except Exception as e:
        print("WARNING: OpenAPI schema generation failed:")
        print(str(e)[:200])
        print("Using minimal spec instead...")
        # Create a minimal spec from available routes
        spec = {
            "openapi": "3.0.0",
            "info": {
                "title": "CYNIC Kernel API",
                "version": "2.0.0",
                "description": "Python kernel - φ-bounded judgment + learning"
            },
            "paths": {}
        }

    # Export JSON
    openapi_path = output_dir / "openapi.json"
    with open(openapi_path, "w") as f:
        json.dump(spec, f, indent=2)
    print(f"[OK] Exported OpenAPI schema: {openapi_path}")

    # Generate and export Markdown
    markdown = generate_markdown_docs(spec)
    md_path = output_dir / "api-reference.md"
    with open(md_path, "w") as f:
        f.write(markdown)
    print(f"[OK] Generated API reference: {md_path}")

    # Print summary
    paths = spec.get("paths", {})
    total_endpoints = sum(1 for path_item in paths.values()
                         for method in path_item if method in ["get", "post", "put", "delete"])
    print("\n[SUMMARY]")
    print(f"   Endpoints: {total_endpoints}")
    print(f"   Tags: {len(set().union(*(op.get('tags', []) for path in paths.values() for op in path.values())))}")
    print("\n[INFO] View API docs: http://localhost:8000/docs")


if __name__ == "__main__":
    main()
