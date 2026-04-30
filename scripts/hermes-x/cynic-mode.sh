#!/usr/bin/env bash
# Switch CYNIC organism mode between staging (visible) and production (headless)

set -euo pipefail

MODE="${1:-}"

case "${MODE}" in
  staging)
    echo "✓ Switching to STAGING: hermes-browser will be visible (headless=false)"
    systemctl --user set-environment CYNIC_ENV=staging
    systemctl --user restart hermes-browser.service 2>/dev/null || echo "  (hermes-browser not running, will use staging on next start)"
    ;;
  production)
    echo "✓ Switching to PRODUCTION: hermes-browser will be headless (headless=true)"
    systemctl --user set-environment CYNIC_ENV=production
    systemctl --user restart hermes-browser.service 2>/dev/null || echo "  (hermes-browser not running, will use production on next start)"
    ;;
  status)
    env | grep CYNIC_ENV || echo "CYNIC_ENV not set (defaults to staging)"
    systemctl --user status hermes-browser.service --no-pager | grep -E "Active|CYNIC_ENV" || true
    ;;
  *)
    echo "Usage: $0 {staging|production|status}"
    echo ""
    echo "  staging   — hermes-browser visible (you monitor organism behavior)"
    echo "  production — hermes-browser headless (organism runs autonomously)"
    echo "  status    — show current mode"
    exit 1
    ;;
esac
