<!-- Describe what changed and why. Link any related issue (e.g. #123). -->

## What & why

## Checklist

- [ ] `cargo fmt --all -- --check` is clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo build --workspace` and `cargo test --workspace --all-features` pass
- [ ] Bug fix? Added a regression test that fails *without* the fix
- [ ] No copyrighted game data added (data, sounds, screenshots); docs updated if needed
- [ ] Any enhancement is **opt-in** and does not break parity with the original
