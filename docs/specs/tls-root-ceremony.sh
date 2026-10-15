#!/usr/bin/env bash
# CYNIC Root CA ceremony (OFFLINE ONLY, HumanAdmin gesture).
# Generates the fleet root CA keypair (ed25519, D-TLS3) and a self-signed
# root certificate. The private key NEVER leaves this machine and is NEVER
# transmitted: copy it to your offline custody yourself.
# Audited helper for tls-fleet-design.md section 5.1 (D-TLS1: manual root).
# Usage (on the offline machine): ./tls-root-ceremony.sh <output-dir>
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <output-dir>" >&2
    exit 1
fi
OUT="$1"
KEY="$OUT/cynic-root-ca.key"
CRT="$OUT/cynic-root-ca.crt"

if [ -e "$KEY" ] || [ -e "$CRT" ]; then
    echo "refusing: key or certificate already exists in $OUT (no overwrite)" >&2
    exit 1
fi
mkdir -p "$OUT"
umask 077

openssl genpkey -algorithm ed25519 -out "$KEY"
chmod 400 "$KEY"
openssl req -x509 -new -nodes -key "$KEY" -sha512 -days 3650 \
    -subj "/CN=CYNIC Fleet Root CA/O=CYNIC Sovereign Fleet" \
    -addext "basicConstraints=critical,CA:TRUE,pathlen:1" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -out "$CRT"

FINGERPRINT="$(openssl x509 -in "$CRT" -noout -fingerprint -sha256 | cut -d= -f2)"
echo "$FINGERPRINT  $(basename "$CRT")" > "$CRT.sha256"
{
    echo "ceremony: CYNIC Fleet Root CA"
    echo "date_utc: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "algorithm: ed25519"
    echo "validity_days: 3650"
    echo "sha256_fingerprint: $FINGERPRINT"
    echo "key_file: $(basename "$KEY") (mode 400, custody: HumanAdmin offline)"
    echo "next: intermediate CSR on kernel primary, signed by this root (D-TLS2)"
} > "$OUT/CEREMONY.txt"

echo "root CA ready in $OUT"
echo "fingerprint: $FINGERPRINT"
echo "guard the key. it never travels."
