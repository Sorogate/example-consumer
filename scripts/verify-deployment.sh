#!/usr/bin/env bash
# Checks that the Sorogate deployment this repository was written against still exists, and that it is the same code as the
# file the tests run against (`tests/fixtures/access_policy.wasm`). It needs the Stellar CLI and sends no transaction.
#
#   bash scripts/verify-deployment.sh
#
# Exit status: 0 the network holds exactly the pinned code; 1 it holds different code, or the fixture is not the pinned file;
# 2 the code could not be fetched (no network, or the contract is gone: Testnet is reset from time to time).
#
# The pinned hash is in `tests/fixtures/access_policy.wasm.sha256`. The tests read the same file, so there is one place to change.
set -euo pipefail

POLICY_CONTRACT=CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW   # Sorogate's public Testnet deployment
FIXTURE=tests/fixtures/access_policy.wasm
PIN=tests/fixtures/access_policy.wasm.sha256

cd "$(dirname "$0")/.."

sha256sum --check --status "$PIN" || { echo "FAIL: $FIXTURE is not the file pinned in $PIN" >&2; exit 1; }
pinned="$(cut -d' ' -f1 "$PIN")"

fetched="$(mktemp)"
trap 'rm -f "$fetched"' EXIT
for attempt in 1 2 3; do
  if stellar contract fetch --id "$POLICY_CONTRACT" --network testnet --out-file "$fetched" 2>/dev/null; then break; fi
  [ "$attempt" = 3 ] && { echo "COULD NOT FETCH $POLICY_CONTRACT from Testnet: the network may be unreachable, or Testnet was reset and the contract is gone" >&2; exit 2; }
  sleep 5
done

actual="$(sha256sum "$fetched" | cut -d' ' -f1)"
if [ "$actual" != "$pinned" ]; then
  echo "FAIL: the network holds different code for $POLICY_CONTRACT" >&2
  echo "  pinned  $pinned" >&2
  echo "  on the network  $actual" >&2
  exit 1
fi
echo "ok: $POLICY_CONTRACT on Testnet is the pinned code ($pinned)"
