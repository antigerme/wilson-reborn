<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Contributing to Wilson Reborn

Thanks for your interest! Wilson Reborn is a faithful, enhanced clone of the 1992 *Johnny
Castaway* screensaver. This guide covers how to build, the quality bar, and how to send
changes. *(Em português? O README tem versão PT: [README.pt-BR.md](README.pt-BR.md).)*

## Ground rules

- **Parity first.** Behavior should match the original (see the
  [knowledge base](docs/knowledge-base/README.md)). Enhancements are welcome, but they must
  be **opt-in** and must not break parity.
- **No game assets, ever.** The original data (`RESOURCE.*`, `SCRANTIC.EXE`, sounds, art) is
  **copyright Sierra/Dynamix** and must **never** be committed (this includes screenshots of
  the running screensaver). Tests use synthetic fixtures so CI runs without any data.
- **License.** By contributing, you agree your work is licensed under **GPL-3.0-or-later**.

## Getting the data (to run/test locally)

The app needs your own copy of the original `RESOURCE.MAP` + `RESOURCE.001`. The screensaver
is preserved at the [Internet Archive](https://archive.org/details/johnny-castaway-screensaver);
`--data` accepts a folder or the `.zip` directly. See [docs/INSTALL.md](docs/INSTALL.md).

## Build & quality bar (must be green)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace
cargo test --workspace --all-features
```

- **Every change builds, lints clean (`-D warnings`) and passes tests** — locally and in CI.
- **Every bug fix comes with a regression test** that fails *without* the fix (verify it
  fails first). This keeps fixed things fixed.
- Match the style of the surrounding code; `wilson-engine` is `#![forbid(unsafe_code)]`.

## Validating behavior

The engine is headless and self-validating — see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
("How to validate"): fast invariants run in CI without data; deep invariants and a sampled
visual montage run locally with your own data.

## Sending changes

1. Branch off `main` (e.g. `feature/...` or `fix/...`).
2. Make the change, with tests and docs.
3. Make sure the quality bar above is green.
4. Open a PR describing **what** changed and **why**; link any related issue. The maintainer
   squash-merges.

## Reporting bugs / ideas

Use the issue templates. For bugs, include your OS, how you ran it (and which options), and
what you saw vs. what you expected. Run with `--debug` for useful logs. **Never attach
copyrighted game data.**
