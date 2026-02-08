#!/usr/bin/env bash
# Optional: run two pea-linux instances and one proxy request (smoke test).
# Use on Linux only. From repo root: ./scripts/interop-two-linux.sh
# Requires: cargo build -p pea-linux (release or debug), curl.
# Discovery between two processes on same host may or may not work (multicast loopback);
# this script mainly checks that two instances start and one proxy request succeeds.

set -e
cd "$(dirname "$0")/.."
BIN="${BIN:-./target/release/pea-linux}"
if [[ ! -x "$BIN" ]]; then
  BIN="./target/debug/pea-linux"
fi
if [[ ! -x "$BIN" ]]; then
  echo "Build pea-linux first: cargo build -p pea-linux [--release]"
  exit 1
fi

cleanup() {
  if [[ -n "$P1" ]]; then kill "$P1" 2>/dev/null || true; fi
  if [[ -n "$P2" ]]; then kill "$P2" 2>/dev/null || true; fi
}
trap cleanup EXIT

# Instance 1: proxy 3128, discovery 45678, transport 45679
PEAPOD_PROXY_PORT=3128 PEAPOD_DISCOVERY_PORT=45678 PEAPOD_TRANSPORT_PORT=45679 "$BIN" &
P1=$!
# Instance 2: proxy 3129, same discovery port (same LAN), transport 45680
PEAPOD_PROXY_PORT=3129 PEAPOD_DISCOVERY_PORT=45678 PEAPOD_TRANSPORT_PORT=45680 "$BIN" &
P2=$!

sleep 3
# One request through first proxy
CODE=$(curl -s -o /dev/null -w "%{http_code}" -x http://127.0.0.1:3128 --connect-timeout 5 http://example.com/ || echo "000")
cleanup
trap - EXIT

if [[ "$CODE" == "200" ]]; then
  echo "OK: two instances started, proxy returned 200"
  exit 0
else
  echo "Proxy returned HTTP $CODE (expected 200)"
  exit 1
fi
