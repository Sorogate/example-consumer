# Sorogate example consumer

The smallest contract that depends on a [Sorogate](https://github.com/Sorogate/sorogate) access policy: a **gate**. It lets an
address in if a policy says it qualifies, and records that it did. It exists to show an independent Stellar developer how to
use Sorogate from their own Soroban contract, with nothing from Sorogate's repository except a published interface.

> **Status: an example. Testnet only. Not audited.** It holds nothing of value, has no administrator, and is not a product.
> Sorogate itself is early, unaudited, and runs as a public *development* deployment on Testnet
> ([`docs/DEPLOYMENT.md`](https://github.com/Sorogate/sorogate/blob/main/docs/DEPLOYMENT.md)); nothing here is for real value.

## How it fits

```
caller ──enter(subject)──▶ gate ──evaluate(policy id, subject)──▶ Sorogate's policy contract (public, Testnet)
                            │                                      reads the subject's balances / the ledger time
                            └─ authenticates the subject, enforces the answer, records the entry
```

Sorogate answers one question, **does this address satisfy this policy right now?** It does not prove who is asking. The gate
therefore does three things, in this order (`src/lib.rs`, about 40 lines of logic):

1. **Authenticate the subject**: `subject.require_auth()`. Without it, anyone could enter on behalf of any qualifying address.
2. **Ask the policy, and fail closed**: if the policy contract cannot answer (wrong address, unknown policy, an error inside it),
   the answer is no.
3. **Enforce the decision**: a denial stops the call with the policy's reason, and nothing is recorded.

Sorogate's interface (`evaluate`, `Decision`, `DenyReason`) is **declared by hand** in `src/lib.rs`, as any consumer would
declare it. This crate does not depend on Sorogate's code.

## Run the tests

```bash
cargo test
```

Needs Rust (the version in `rust-toolchain.toml` is installed by rustup on first use) and the `wasm32v1-none` target.
**On Windows, run it in WSL**: native linking of soroban-sdk's test utilities fails.

The tests run against **the real policy contract**, not a mock. `tests/fixtures/access_policy.wasm` is the code the network holds
for Sorogate's public deployment, fetched with the standard CLI:

```bash
stellar contract fetch --id CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW --network testnet \
  --out-file tests/fixtures/access_policy.wasm
sha256sum tests/fixtures/access_policy.wasm   # f702e9d267262ab3f9548fa62021b4c31bb5b5f4f3652ff713906dcd48906814
```

A test checks that hash, so the fixture cannot silently stop being the deployed contract. Because the tests import the contract's
own interface from that file, a hand-declared interface that drifts from the real one fails them. They cover: an address that
qualifies is let in; one below the minimum is refused with `BelowMinimum` and nothing is recorded; an address holding nothing is
refused; a qualifying address is **not** let in without its own signature; the owner changing the rule applies at once with no
redeploy; moving tokens away removes eligibility; a deactivated policy lets nobody in; and an unknown policy or a wrong address is
a refusal, not a pass.

## Try it on Testnet

```bash
stellar contract build
bash scripts/testnet-demo.sh
```

It uses only the Stellar CLI: two throwaway accounts, a policy on Sorogate's public contract ("hold at least 10,000 XLM", over
the native asset's contract), the gate deployed in front of it, then one account let in and the other refused. Keys live in a
temporary directory that is deleted afterwards. A recorded run, with the real addresses and what kind of result each line is, is in
[`evidence/`](evidence).

## Use it with your own policy

1. **Make a policy** on Sorogate's contract (Testnet), with the CLI as in the demo script, or with Sorogate's TypeScript SDK.
   `stellar contract invoke --id CACR5H46… -- create --help` prints the conditions it accepts, from the contract's own interface.
   Amounts are in a token's smallest unit.
2. **Deploy the gate** with that policy's id: `stellar contract deploy --wasm … -- --policy_contract CACR5H46… --policy_id <id>`.
3. **Call `enter`** as the subject, signing as that address.

For another contract of your own, copy the pattern in `src/lib.rs`, and read Sorogate's
[integration guide](https://github.com/Sorogate/sorogate/blob/main/docs/INTEGRATING.md): pin a policy version if the rules must not
change under you, and read what the guide says goes wrong.

## What this does not show

- It is **one** policy shape over one token (native XLM). Sorogate's other conditions (`NftBalance`, `TimeWindow`) and policies
  of several conditions are covered in Sorogate's own tests and evidence, not here.
- The demo's refusals are **simulations**, which cannot be applied. Sorogate's evidence records refusals that were applied and
  failed on the network, with their error codes.
- It uses a **development deployment on Testnet**, which is reset from time to time. If the contract named above no longer
  exists, the demo and the policy it created are gone too.
- Nothing here has been used by anyone else, and it is not a statement about Sorogate's security.

## License

Apache-2.0. Not affiliated with or endorsed by the Stellar Development Foundation; "Stellar" and "Soroban" are their trademarks.
