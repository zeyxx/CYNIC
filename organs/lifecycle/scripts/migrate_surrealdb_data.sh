#!/usr/bin/env bash
set -e

echo "=== SurrealDB Data Migration ==="
echo "Exporting data from local instance (cynic-vm-v2)..."

# Assuming local surrealdb is running on 127.0.0.1:8000 with root:root
surreal export --conn http://127.0.0.1:8000 --user root --pass root --ns test --db test migration_export.surql

echo "Export successful."
echo "Wait for the new container (192.168.0.43) to be fully running..."
sleep 2

echo "Importing data to new LXC instance (cynic-lxc-surrealdb at 192.168.0.43)..."
surreal import --conn http://192.168.0.43:8000 --user root --pass root --ns test --db test migration_export.surql

echo "Import successful."
echo "Migration complete! Please update your .cynic-env or application environment to point to 192.168.0.43:8000"
