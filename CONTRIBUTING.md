# Contributing

This repository is an example: the smallest correct contract that uses a [Sorogate](https://github.com/Sorogate/sorogate)
access policy. It stays small on purpose, and the best contributions keep it that way.

## Welcome

- **Fixes to the gate or its tests** where the example is wrong, unclear, or does not show a boundary it claims to.
- **A test for a behaviour the README claims** and no test checks, or a clearer name for a test.
- **Corrections to the README or the recorded evidence**: a stale address, a wrong command, a missing step for someone starting
  from nothing.
- **Reports that the instructions do not work** on your machine, with the command and what it printed.

## Not welcome

This is not going to become an application. No frontend, wallet, marketplace, token-gated site, payment flow, indexer, SDK, or
credential system, and no second implementation of Sorogate. Features of Sorogate itself go in
[the Sorogate repository](https://github.com/Sorogate/sorogate/issues). A change that makes this repository import Sorogate's
source is out of scope: that the gate does not is the point.

## Picking up an issue

Open issues are on the [issues page](https://github.com/Sorogate/example-consumer/issues). Comment on one to say you would like
it, and wait for the maintainer to assign it to you before you start. A comment alone does not reserve it. If an assigned issue
has had no activity for 7 days, the maintainer may ask whether you are still working on it, and may unassign it after 7 more
days without a reply.

The first time you open a pull request, GitHub holds its CI run until a maintainer approves it. That is a GitHub setting, not
broken CI. AI-assisted contributions are welcome: you are responsible for what you submit, you have run it, and every claim in
the description is true.

## Run the checks

You need Rust (rustup installs the pinned version) and the [Stellar CLI](https://github.com/stellar/stellar-cli) 25.2 or newer. On
Windows, use WSL.

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
stellar contract build --locked        # the gate must be built with this, not plain cargo
bash scripts/verify-deployment.sh      # needs the network; see the README for its exit codes
```

CI runs the same. Its actions are pinned by commit hash; if you change one, change the comment beside it too, and keep
`version:` under the Stellar CLI step equal to the release that pin names.

## Changing the gate

Keep `subject.require_auth()` first, keep every failure to get an answer a refusal, and enforce the decision. A change to those
three needs a test that fails without it.

## The recorded Testnet run

`scripts/testnet-demo.sh` makes everything it needs with throwaway keys, and prints one line per step, each labelled as a
transaction, a simulation, or a reading. To record a new run: build the gate, run the script, save its output as
`evidence/testnet-<date>.txt`, and update `evidence/README.md` from it (addresses, transaction hashes, what was expected and what
happened). Never commit a key. Each run leaves a policy on the public Sorogate contract owned by a discarded key; say so.

## Moving to a different Sorogate deployment

If Sorogate publishes a new deployment, update these together, and nothing else should need to change:

1. The contract address in `scripts/verify-deployment.sh`, `scripts/testnet-demo.sh` and the README.
2. `tests/fixtures/access_policy.wasm`, fetched with the command in the README, and its hash in
   `tests/fixtures/access_policy.wasm.sha256`.
3. The interface in `src/lib.rs`, if the new contract changed it. The tests will fail if the declaration no longer matches.
4. The recorded run, if you can make a new one.

## Security

Do not open a public issue for a security problem; see the organization's
[security policy](https://github.com/Sorogate/.github/blob/main/SECURITY.md). This is a Testnet example with no administrator and
nothing of value, so most reports will concern Sorogate itself, and belong there.

## Licence

By contributing you agree that your contribution is licensed under Apache-2.0, as the rest of the repository is.
