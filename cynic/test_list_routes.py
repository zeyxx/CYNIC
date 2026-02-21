"""Quick script to list all routes in FastAPI app."""

from cynic.api.server import app

print("\n" + "="*70)
print("FastAPI ROUTES")
print("="*70 + "\n")

for route in app.routes:
    if hasattr(route, "path"):
        methods = getattr(route, "methods", ["GET"])
        print(f"{route.path:40} {methods}")

print("\n" + "="*70)
