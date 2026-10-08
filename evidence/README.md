# A recorded run on Testnet, 2026-10-08

**Label: Recorded.** One run of [`scripts/testnet-demo.sh`](../scripts/testnet-demo.sh), the Stellar CLI only, against Sorogate's
**public Testnet development deployment**. The raw output is [`testnet-2026-10-08.txt`](testnet-2026-10-08.txt). Every line is
labelled with the kind of result it is. An [earlier run](testnet-2026-10-07.txt) of the script, before it checked its own results
(see the end of this page), gave the same outcomes.

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
| Policy | id `18`: hold at least 10,000 XLM, over the native asset's contract `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`. Created by `alice` in transaction `245db8c7a9c65be12c4c3c7dac611fb9bd22de3735666fee7785b36478f03d05` |
| Gate | `CBG4R4OKTOJZMVEDTU7L7JJR2WBJHLNN4QBFDUOUJFEWF3DCX4BAWQVI`, deployed in transaction `5dfb39c8a4e2c45181e590e6387119f8c6e7d350fff96657d01b9d107c39c8e5`. 3,544 bytes, sha256 `9885f2a8b5a0710299a50391fb36667da0b3c856dbadd8508a48f29095bb48d6`, built with stellar-cli 27.1.0 |
| Accounts | `alice` `GC56UNLPCJIDRJQ3OW3HNTKR3UYNMT6HL2NN36E3VAD32NNCMLZDZBEV` (qualifying), `bob` `GC5UKGQLGPI7IL2TW3L273BM3RGKKNL3DL3YN4Y6A6MI2EA36BG6CFHQ` (denied). **Throwaway** Testnet accounts, funded by friendbot. Their keys were in a temporary directory that the script deleted, so nobody holds them |

Alice is the **qualifying** account and Bob the **denied** one. Four transactions were submitted: Bob's transfer, the policy's
creation, the gate's deployment and Alice's entry (the CLI also uploaded the gate's code in a transaction of its own, whose hash
this run did not capture), besides friendbot funding the two accounts. Everything else is a simulation or a reading. The script
checks each result against what is expected, and stops with `UNEXPECTED` if one differs; the last column is what it found.

| Step | Kind | Expected | Actual |
| --- | --- | --- | --- |
| Bob sends Alice 5,000 XLM, so Alice holds about 15,000 and Bob about 5,000 | transaction | applied | applied: `83dc3f12bab18dbf943b2752a1076768b1a429b087687be6fc62357a9f5d076e` |
| Sorogate's own answer for Alice | simulation | allowed | allowed (`{"allowed":true,"failed_index":null,"reason":0,"version":1}`) |
| Sorogate's own answer for Bob | simulation | denied, below the minimum | denied, `reason` 2 (`BelowMinimum`), failing condition 0 |
| Alice enters the gate | transaction | let in and recorded | let in: `3b994df05e1c612f6c5ed89fee300453fdfb91ef365b5d83a4505a85540be9ed`. `is_member(Alice)` reads `true` |
| Bob enters the gate | simulation | refused with the policy's reason, nothing recorded | refused with the gate's `Error(Contract, #11)`, `BelowMinimum`. `is_member(Bob)` reads `false` |
| Bob tries to enter on Alice's behalf, with only Bob's key available | refused before sending | not accepted without Alice's signature | the CLI: `Missing signing key for account GC56UNLPCJIDRJQ3OW3HNTKR3UYNMT6HL2NN36E3VAD32NNCMLZDZBEV` |

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
- **Earlier runs left policies on the public contract.** Policies 9 to 12 came from earlier runs of this script on 2026-10-07,
  stopped or changed for script bugs (the unexpected acceptance above, a command that needed `--force`, a hash-capture
  change). Policy 13 is the [2026-10-07 transcript](testnet-2026-10-07.txt), made by the script before it checked its own
  results: it printed what it found, and only two outcomes could stop it. Policy 18 is this transcript, made after every expected
  outcome became a check. The numbers between belong to other runs on the same contract. All are owned by discarded keys, so they
  cannot be changed or deactivated.
- **What a refusal records.** A refused `enter` fails, and a failed call leaves nothing written, so `is_member` stays `false`. The
  tests check this, and that a successful entry is a record of what happened, not a live entitlement: it stays when the rule later
  changes.
- **One policy, one token.** One `TokenBalance` condition over native XLM. Sorogate's other conditions are not exercised here.
- **A development deployment.** Testnet is reset from time to time; if the contract above no longer exists, neither do the policy,
  the gate and these accounts.
- It is one run. It found nothing wrong with Sorogate, which does not show that nothing is wrong.
