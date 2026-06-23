#!/usr/bin/env bash
# SSOT endpoint resolver for kernel REST address.
#
# Requires:
#   CYNIC_REST_ADDR in environment (or source ~/.cynic-env)
#
# Accepted values:
#   - <HEADSCALE_CORE>:3030
#   - http://<HEADSCALE_CORE>:3030
#   - https://...
#
# Policy:
#   local loopback endpoints (127.0.0.1/localhost) are rejected by default to avoid
#   accidental divergence between operators. Set CYNIC_ALLOW_LOCAL_KERNEL=1 only for
#   explicit dev exceptions.
cynic_kernel_rest_url() {
    if [[ -z "${CYNIC_REST_ADDR-}" && -f "${HOME}/.cynic-env" ]]; then
        set -a
        # shellcheck disable=SC1090
        source "${HOME}/.cynic-env"
        set +a
    fi

    local addr="${CYNIC_REST_ADDR:-}"
    if [[ -z "$addr" ]]; then
        echo "FATAL: CYNIC_REST_ADDR is required (source ~/.cynic-env). Missing for SSOT endpoint resolution." >&2
        return 1
    fi

    if [[ "$addr" != http://* && "$addr" != https://* ]]; then
        addr="http://$addr"
    fi

    local hostport="${addr#http://}"
    hostport="${hostport#https://}"
    hostport="${hostport%%/*}"
    local host="${hostport%%:*}"

    if [[ "$host" == "127.0.0.1" || "$host" == "localhost" ]] && [[ "${CYNIC_ALLOW_LOCAL_KERNEL:-0}" != "1" ]]; then
        echo "FATAL: SSOT rejected loopback kernel endpoint '$addr'. Set CYNIC_REST_ADDR to fleet endpoint (e.g. <HEADSCALE_CORE>:3030)." >&2
        return 1
    fi

    printf '%s\n' "$addr"
}
