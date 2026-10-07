# A recorded run on Testnet, 2026-10-07

**Label: Recorded.** One run of [`scripts/testnet-demo.sh`](../scripts/testnet-demo.sh), the Stellar CLI only, against Sorogate's
**public Testnet development deployment**. The raw output is [`testnet-2026-10-07.txt`](testnet-2026-10-07.txt). Every line is
labelled with the kind of result it is.

| Label | Meaning |
| --- | --- |
| **transaction** | Sent to the network and applied |
| **simulation** | Asked of the network without applying anything: the same code runs, nothing happens on the ledger |
| **reading** | A value looked up |
| **refused before sending** | The Stellar CLI declined to build the call: nothing reached the network |

## What happened

| | |
| --- | --- |
| Network | Stellar Testnet, protocol 29 |
| Sorogate policy contract | `CACR5H46E7VKEDJUWRLKZPJPQVPRHTRTJHZOMQLEIHMTXN4MW7O47YQW`, code sha256 `f702e9d2…8906814` (the file in `tests/fixtures/`) |
| Policy | id `13`: hold at least 10,000 XLM, over the native asset's contract `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`. Created by `alice` in transaction `02929cad52212dd26e5db16fbc3c574c6833f0e37847f889aec95776efe7c944` |
| Gate | `CBXNFJXLO6VF2TETVZAEKWHHBFP4OMOKXOSDCFXHK2ENVFL3WUDPKMNX`, deployed in transaction `29ac631c1bb64787b27a8abbd2979c06206e55f2c38e902d2b0c87b13b8ecb71`. 3,544 bytes, sha256 `9885f2a8b5a0710299a50391fb36667da0b3c856dbadd8508a48f29095bb48d6`, built with stellar-cli 27.1.0 |
| Accounts | `alice` `GBKKOBEPYCQHW42R5MQ5LZ65MTX7NKWXNNI5M22J4F2PQRWZ5CCJ6N5S`, `bob` `GCOU5Q2K2R2BPA2DL5FTLAAJ77FKB2FMK5PAQ57VX4BW2JFKMD5LAFQU`. Throwaway, funded by friendbot. Their keys were in a temporary directory that the script deleted |

| Step | Kind | Result |
| --- | --- | --- |
| Bob sends Alice 5,000 XLM, so Alice holds about 15,000 and Bob about 5,000 | transaction | `dc4f1919b45fb4b7649bd2b52380e1a26ef2e6dff55fea83cd90aae0738fe012` |
| Sorogate's own answer for Alice | simulation | allowed (`{"allowed":true,"failed_index":null,"reason":0,"version":1}`) |
| Sorogate's own answer for Bob | simulation | denied, `reason` 2 (`BelowMinimum`), failing condition 0 |
| Alice enters the gate | transaction | let in: `e76591753b96064ceea4285c0130020e867dc65dca2a2421bda7fdb004ed05c7`. `is_member(Alice)` reads `true` |
| Bob enters the gate | simulation | refused with the gate's `Error(Contract, #11)`, `BelowMinimum`. `is_member(Bob)` reads `false` |
| Bob tries to enter on Alice's behalf, with only Bob's key available | refused before sending | the CLI: `Missing signing key for account GBKKOBEPYCQHW42R5MQ5LZ65MTX7NKWXNNI5M22J4F2PQRWZ5CCJ6N5S` |

So a separate contract, with nothing from Sorogate's repository except its published interface, asked the public deployment about
two addresses, **enforced** the answers (one let in and recorded, one refused with the policy's reason), and the standard tools
could create the policy, deploy the gate and call it.

## What it does not show, and what went wrong on the way

- **The refusals here are simulations**, which cannot be applied, and the last step was refused by the CLI, not by the network.
  Refusals that were **applied and failed on the network**, with their error codes, including a claim signed by someone else,
  are recorded in [Sorogate's own evidence](https://github.com/Sorogate/sorogate/blob/main/docs/evidence/testnet-gated-claim-2026-10-07.md)
  for its reference consumer. This repository's tests check the same behaviour against the real contract, but do not apply
  anything on a network.
- **The Stellar CLI signs for any account whose key it holds.** In an earlier run of this script, Alice's key was still in the
  temporary configuration, and `enter --subject <Alice>` with Bob as the source was **accepted**: the CLI had signed Alice's
  authorization itself, so it was an authorized call, not a bypass. The script now removes Alice's key first, so the last step
  means what it says. Anyone reproducing this should know the CLI does that.
- **Earlier runs left policies on the public contract.** Policies 9 to 12 came from earlier runs of this script the same day,
  stopped or changed for script bugs (the unexpected acceptance above, a command that needed `--force`, a hash-capture
  change). Policy 13 is this transcript. All are owned by discarded keys, so they cannot be changed or deactivated.
- **One policy, one token.** One `TokenBalance` condition over native XLM. Sorogate's other conditions are not exercised here.
- **A development deployment.** Testnet is reset from time to time; if the contract above no longer exists, neither do the policy,
  the gate and these accounts.
- It is one run. It found nothing wrong with Sorogate, which does not show that nothing is wrong.
