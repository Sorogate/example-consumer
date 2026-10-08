#!/usr/bin/env bash
# Puts the gate on Stellar Testnet in front of Sorogate's public policy contract, and tries it, using only the Stellar CLI.
#
#   stellar contract build
#   bash scripts/testnet-demo.sh [path/to/sorogate_example_gate.wasm]
#
# What it does:
#   1. makes two throwaway Testnet accounts, "alice" and "bob", and moves XLM so that alice holds more than 10,000 and bob less
#   2. creates a policy on Sorogate's public contract: "hold at least 10,000 XLM" (the native asset's Stellar asset contract)
#   3. deploys the gate, pointed at that policy
#   4. asks the policy contract about each account, then asks the gate to let each one in
#
# Every line says what kind of result it is: a TRANSACTION was sent and applied; a SIMULATION was asked of the network and
# changed nothing; a READING is a value looked up.
#
# The throwaway keys live in a temporary directory that is deleted when the script ends. Nothing is written to your own Stellar
# CLI configuration. It talks only to Testnet: every command names `--network testnet`.
set -euo pipefail

POLICY_CONTRACT=CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW   # Sorogate's public Testnet deployment
GATE_WASM="${1:-target/wasm32v1-none/release/sorogate_example_gate.wasm}"
MIN_STROOPS=100000000000   # 10,000 XLM; 1 XLM is 10,000,000 stroops

[ -f "$GATE_WASM" ] || { echo "no WASM at $GATE_WASM: run 'stellar contract build' first" >&2; exit 1; }

export XDG_CONFIG_HOME="$(mktemp -d)"
trap 'rm -rf "$XDG_CONFIG_HOME"' EXIT
NET=(--network testnet)

say() { printf '%s\n' "$*"; }

# Stops the script if a result is not the expected one: `expect <what> <the result> <a pattern it must match>`.
expect() {
  printf '%s' "$2" | grep -qE "$3" || { say "[UNEXPECTED] $1: got '$2', expected something matching '$3'"; exit 1; }
}

# Runs a command, keeps what it printed in TX_OUT, and the hashes of the transactions it sent (the CLI logs them) in TX_HASHES.
tx() {
  local err
  err=$(mktemp)
  TX_OUT=$("$@" 2>"$err") || { cat "$err" >&2; rm -f "$err"; return 1; }
  TX_HASHES=$(grep -oE 'Signing transaction: [0-9a-f]{64}' "$err" | awk '{print $3}' | paste -sd, -)
  rm -f "$err"
}

say "== setting up (two throwaway accounts, funded by friendbot)"
stellar keys generate alice "${NET[@]}" --fund >/dev/null 2>&1
stellar keys generate bob "${NET[@]}" --fund >/dev/null 2>&1
ALICE=$(stellar keys address alice)
BOB=$(stellar keys address bob)
NATIVE=$(stellar contract id asset --asset native "${NET[@]}")
say "alice  $ALICE"
say "bob    $BOB"
say "native asset contract  $NATIVE"

# Friendbot gives both 10,000 XLM. Bob sends alice 5,000, so alice has about 15,000 and bob about 5,000.
tx stellar tx new payment --source-account bob --destination "$ALICE" --asset native --amount 50000000000 "${NET[@]}"
say "[transaction] bob sent alice 5,000 XLM (tx $TX_HASHES)"

say
say "== a policy on Sorogate's public contract: hold at least 10,000 XLM"
tx stellar contract invoke --id "$POLICY_CONTRACT" --source-account alice "${NET[@]}" -- create \
  --owner "$ALICE" --conditions "[{\"TokenBalance\":{\"token\":\"$NATIVE\",\"min\":\"$MIN_STROOPS\"}}]"
POLICY_ID=$(printf '%s' "$TX_OUT" | tail -1 | tr -d '"')
say "[transaction] policy $POLICY_ID created on $POLICY_CONTRACT, owned by alice (tx $TX_HASHES)"

say
say "== the gate, pointed at that policy"
tx stellar contract deploy --wasm "$GATE_WASM" --source-account alice "${NET[@]}" -- \
  --policy_contract "$POLICY_CONTRACT" --policy_id "$POLICY_ID"
GATE=$(printf '%s' "$TX_OUT" | tail -1)
say "[transaction] gate deployed at $GATE (tx $TX_HASHES; the CLI also uploads the code first, in a transaction of its own, unless it is already on the network)"

say
say "== what Sorogate says about each account (the policy contract's own answer)"
ALICE_ANSWER=$(stellar contract invoke --id "$POLICY_CONTRACT" --source-account alice "${NET[@]}" --send=no -- evaluate --id "$POLICY_ID" --subject "$ALICE" 2>/dev/null | tail -1)
BOB_ANSWER=$(stellar contract invoke --id "$POLICY_CONTRACT" --source-account alice "${NET[@]}" --send=no -- evaluate --id "$POLICY_ID" --subject "$BOB" 2>/dev/null | tail -1)
say "[simulation] alice: $ALICE_ANSWER"
say "[simulation] bob:   $BOB_ANSWER"
expect "Sorogate's answer for alice" "$ALICE_ANSWER" '"allowed":true'
expect "Sorogate's answer for bob" "$BOB_ANSWER" '"allowed":false.*"reason":2'   # 2 is BelowMinimum

say
say "== the gate enforces it"
tx stellar contract invoke --id "$GATE" --source-account alice "${NET[@]}" -- enter --subject "$ALICE"
say "[transaction] alice enters: let in (tx $TX_HASHES)"
ALICE_MEMBER=$(stellar contract invoke --id "$GATE" --source-account alice "${NET[@]}" -- is_member --subject "$ALICE" 2>/dev/null | tail -1)
say "[reading] alice is a member: $ALICE_MEMBER"
expect "alice's membership" "$ALICE_MEMBER" '^true$'

if refused=$(stellar contract invoke --id "$GATE" --source-account bob "${NET[@]}" -- enter --subject "$BOB" 2>&1); then
  say "[UNEXPECTED] bob was let in"; exit 1
else
  BOB_ERROR=$(printf '%s' "$refused" | grep -oE 'Error\(Contract, #[0-9]+\)' | head -1)
  say "[simulation] bob enters: refused -> $BOB_ERROR  (11 is BelowMinimum)"
  expect "the gate's refusal of bob" "$BOB_ERROR" '#11\)'
fi
BOB_MEMBER=$(stellar contract invoke --id "$GATE" --source-account bob "${NET[@]}" -- is_member --subject "$BOB" 2>/dev/null | tail -1)
say "[reading] bob is a member: $BOB_MEMBER"
expect "bob's membership" "$BOB_MEMBER" '^false$'

say
say "== eligibility is not identity: bob tries to enter on behalf of alice, who qualifies but did not sign"
# The CLI signs for any account whose key it holds, so alice's key is removed first: from here only bob can sign.
stellar keys rm alice --force >/dev/null 2>&1
say "(alice's key removed from the CLI's configuration; only bob can sign now)"
if forged=$(stellar contract invoke --id "$GATE" --source-account bob "${NET[@]}" -- enter --subject "$ALICE" 2>&1); then
  say "[UNEXPECTED] a call signed by bob was accepted for alice"; exit 1
else
  say "[refused before sending] bob signs, alice has not: $(printf '%s' "$forged" | grep -iE 'error|sign|auth|missing' | head -2 | tr '\n' ' ')"
fi

say
say "done. gate $GATE, policy $POLICY_ID on $POLICY_CONTRACT"
