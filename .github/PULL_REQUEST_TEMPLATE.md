## What this changes

## Why

## Checklist
- [ ] `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked` pass
- [ ] `stellar contract build --locked` passes
- [ ] A test fails if the change is undone (break the code on purpose and watch it fail)
- [ ] The gate still imports nothing from Sorogate's source, and still authenticates, fails closed, and enforces the decision
- [ ] Any documented claim this affects is updated, and rests on a test or a recorded run
- [ ] Linked issue, if one exists: Closes #
