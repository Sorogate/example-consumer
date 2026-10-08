# Sorogate example consumer

The smallest contract that depends on a [Sorogate](https://github.com/Sorogate/sorogate) access policy: a **gate**. It lets an
address in if a policy says it qualifies, and records that it did. **Sorogate is the reusable primitive; this repository is an
independent example of using it from your own contract.** It is not an official or production integration.

> **Status: an example. Testnet only. Not audited.** It holds nothing of value, has no administrator, and is not a product.
> Sorogate itself is early, unaudited, and runs as a public *development* deployment on Testnet
> ([`docs/DEPLOYMENT.md`](https://github.com/Sorogate/sorogate/blob/main/docs/DEPLOYMENT.md)); nothing here is for real value.
> Nobody outside the Sorogate project has used this.

## What it calls

| | |
| --- | --- |
| Sorogate policy contract | `CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW` on **Stellar Testnet** ([explorer](https://stellar.expert/explorer/testnet/contract/CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW)) |
| Its code, pinned | sha256 `f702e9d267262ab3f9548fa62021b4c31bb5b5f4f3652ff713906dcd48906814`, kept in [`tests/fixtures/access_policy.wasm.sha256`](tests/fixtures/access_policy.wasm.sha256) |
| The one function it uses | `evaluate(id: u64, subject: Address) -> Decision`, with `Decision { allowed, version, failed_index, reason }` |

## Why this is a separate repository

This repository **intentionally does not import Sorogate's implementation.** It demonstrates the integration boundary that an
independent Soroban developer would consume: a contract address and an interface. Its only dependency is `soroban-sdk`.

- **The interface is declared by hand** in [`src/lib.rs`](src/lib.rs): one function and two small types, as any consumer would
  write it. Depending on Sorogate's crate instead would tie this code to Sorogate's source and hide where the boundary is.
- **A hand-written interface can drift, so the tests check it.** They run against the real policy contract, imported from
  `tests/fixtures/access_policy.wasm`, the code the network holds for the deployment above, fetched with the standard Stellar CLI.
  That file is a **test fixture only**: a compiled binary, with no Sorogate source, and the gate never links to it.

## What the gate does

```
caller ──enter(subject)──▶ gate ──evaluate(policy id, subject)──▶ Sorogate's policy contract (public, Testnet)
                            │                                      reads the subject's balances / the ledger time
                            └─ authenticates the subject, enforces the answer, records the entry
```

Sorogate answers one question: **does this address satisfy this policy right now?** It does not prove who is asking, so the
gate does three things, in this order (`src/lib.rs`, about 40 lines of logic):

1. **Authenticate the subject**: `subject.require_auth()`. Without it, anyone could enter on behalf of any qualifying address.
2. **Ask the policy, and fail closed**: if the policy contract cannot answer (wrong address, unknown policy, a failure inside it,
   an answer that is not a decision), the answer is no.
3. **Enforce the decision**: a denial stops the call with the policy's reason, and nothing is recorded.

## Run it, from nothing

You need Rust (rustup installs the version in `rust-toolchain.toml`, with the `wasm32v1-none` target, on first use) and the
[Stellar CLI](https://github.com/stellar/stellar-cli), 25.2 or newer (CI uses 28.1.0). **On Windows, use WSL**: native linking of
soroban-sdk's test utilities fails.

```bash
git clone https://github.com/Sorogate/example-consumer.git && cd example-consumer
bash scripts/verify-deployment.sh   # does Testnet still hold the pinned Sorogate code?
cargo test --locked                 # the tests, against the real policy contract
stellar contract build --locked     # the gate's WASM
bash scripts/testnet-demo.sh        # the same gate on Testnet, with two throwaway accounts
```

The demo looks for the gate at `target/wasm32v1-none/release/sorogate_example_gate.wasm`. If you build somewhere else (for
example with `CARGO_TARGET_DIR` set), pass the path: `bash scripts/testnet-demo.sh path/to/sorogate_example_gate.wasm`.

`verify-deployment.sh` fetches the contract with `stellar contract fetch` and compares its hash with the pin. It exits `0` when
they match, `1` when the network holds different code, and `2` when it cannot fetch (no network, or Testnet was reset and the
contract is gone). The tests need no network: they use the committed fixture, and a test checks that file against the pin.

**What the tests cover**, each against the real contract:

| Behaviour | Test |
| --- | --- |
| A qualifying subject who authorizes is let in, and recorded | `an_address_that_qualifies_and_authorizes_is_let_in_and_recorded` |
| A subject below the minimum, or holding nothing, is refused and not recorded | `an_address_below_the_minimum_…`, `an_address_that_holds_nothing_…` |
| Nobody else can enter for a qualifying address, and with no authorization at all it is refused | `nobody_can_enter_on_a_qualifying_addresss_behalf`, `with_no_authorization_at_all_…` |
| The subject is authenticated before the policy is asked | `the_subject_is_authenticated_before_the_policy_is_asked` |
| The owner raising or lowering the minimum applies at once, for the same subject | `raising_the_minimum_…`, `lowering_the_minimum_…` |
| Moving tokens away removes eligibility | `a_subject_let_in_is_refused_after_moving_tokens_away` |
| A deactivated policy lets nobody in | `a_deactivated_policy_lets_nobody_in` |
| Fail closed: unknown policy, not a contract, a contract that is not a policy, a policy that fails, an answer that is not a decision, a no without a reason | `a_policy_that_does_not_exist_…` and the five tests after it |
| The hand-declared codes are the real contract's | `the_reason_codes_declared_here_are_the_real_contracts` |

An entry is a **record of what happened**, not a live entitlement: it stays when the rule later changes
(`an_entry_that_was_recorded_stays_recorded_when_the_rule_changes`). A caller that must stop honouring someone has to ask the
policy again.

## The Testnet demo: what it proves and what it does not

`scripts/testnet-demo.sh` uses only the Stellar CLI. It makes two throwaway accounts, creates a policy on Sorogate's public
contract ("hold at least 10,000 XLM", over the native asset's contract), deploys the gate in front of it, and lets one account in
and refuses the other. It stops with `UNEXPECTED` if any result is not the expected one. Keys live in a temporary directory that is
deleted afterwards. The recorded run, with every address, transaction, and what kind of result each line is, is in
[`evidence/`](evidence).

- **It shows** that a separate contract, using only the published interface, asked the public deployment about two addresses and
  enforced the answers, with the policy's reason for the refusal.
- **It does not show** refusals that were *applied and failed* on the network: the demo's refusals are simulations, and the
  forged-caller step is refused by the CLI before anything is sent. Sorogate's own
  [evidence](https://github.com/Sorogate/sorogate/blob/main/docs/evidence/testnet-gated-claim-2026-10-07.md) records those. The
  tests above check the same behaviour against the real contract.
- **It does not show** other conditions (`NftBalance`, `TimeWindow`), policies of several conditions, or anything about
  Sorogate's security.

## Use it with your own policy

1. **Make a policy** on Sorogate's contract (Testnet), with the CLI as in the demo script, or with Sorogate's TypeScript SDK.
   `stellar contract invoke --id CACR5H46… -- create --help` prints the conditions it accepts, from the contract's own interface.
   Amounts are in a token's smallest unit.
2. **Deploy the gate** with that policy's id: `stellar contract deploy --wasm … -- --policy_contract CACR5H46… --policy_id <id>`.
3. **Call `enter`** as the subject, signing as that address.

For a contract of your own, copy the pattern in `src/lib.rs`, and read Sorogate's
[integration guide](https://github.com/Sorogate/sorogate/blob/main/docs/INTEGRATING.md): pin a policy version if the rules must
not change under you, and read what the guide says goes wrong.

## Compatibility

| | |
| --- | --- |
| This repository | version `0.0.0`, untagged: the current `main` |
| Sorogate interface | **No formal interface version exists yet.** The assumption is the one above: `evaluate(u64, Address) -> Decision`, with the reason codes `None 0, Inactive 1, BelowMinimum 2, BalanceUnavailable 3, BeforeWindow 4, AfterWindow 5` |
| Sorogate deployment | the contract above, identified by its code hash: it has no on-chain version function |
| What keeps them in step | the tests import the real contract's interface from the fixture, so a drift fails them, and `verify-deployment.sh` (also run in CI, weekly) fails if the network no longer holds the pinned code |

Sorogate's policy contract has no administrator and no upgrade path, so this deployment cannot change under the gate. A new
version of Sorogate would be a **new contract at a new address with a new hash**; this repository would move to it only by an
explicit change (see [`CONTRIBUTING.md`](CONTRIBUTING.md)). If Testnet is reset, the contract disappears: `verify-deployment.sh`
exits `2`, and the tests keep passing, because they use the committed fixture.

## Limits

- **One policy shape** over one token (native XLM) is demonstrated. The tests use a Stellar asset contract, not other tokens.
- **A development deployment**, reset from time to time. If the contract above no longer exists, neither do the policies, the gate
  and the accounts in the recorded run.
- Nothing here has been used by anyone else, and it is not a statement about Sorogate's security.

## Contributing and security

A short [`CONTRIBUTING.md`](CONTRIBUTING.md) says what changes are welcome here. Security problems go through
[private vulnerability reporting](https://github.com/Sorogate/example-consumer/security/advisories/new); see the organization's
[security policy](https://github.com/Sorogate/.github/blob/main/SECURITY.md).

## License

Apache-2.0. Not affiliated with or endorsed by the Stellar Development Foundation; "Stellar" and "Soroban" are their trademarks.
