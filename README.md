<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Wilson Reborn

A **modern, portable, enhanced** clone of the classic **Johnny Castaway** screensaver
(Sierra/Dynamix, 1992) — *"the first screen saver that tells a story."*

The goal is to bring Johnny back with **full parity** — every gag, event, story beat,
easter egg, holiday and behavior of the original — running on **Windows, Linux and macOS**
at **modern resolutions**, with optional enhancements, without losing anything from the
original.

> 🌎 Em português: **[README.pt-BR.md](README.pt-BR.md)**

## Demo

![Johnny fishing off his island — Wilson Reborn](docs/screenshot.png)

![Wilson Reborn in motion — fishing as the clouds drift](docs/demo.gif)

*Live renders from the Wilson Reborn engine, driven by the original 1992 data.* The original
screensaver is also preserved at the
[Internet Archive](https://archive.org/details/johnny-castaway-screensaver).

> The artwork is © Sierra/Dynamix, shown here only to illustrate the project. This repository
> ships **no game data** — build and run with your own copy (below) to watch Johnny live.

## Running

Wilson Reborn uses the **original** Johnny Castaway data files (`RESOURCE.MAP` +
`RESOURCE.001`) — there is no bundled art. It looks for the data in this order: `--data
<dir>` → `$WILSON_DATA_DIR` → current directory → next to the executable (and in a `data/`
subfolder of each). Without the data it prints where it looked and exits.

```bash
cargo run -p wilson -- --data <dir>              # your original RESOURCE.MAP/RESOURCE.001
cargo run -p wilson -- --data <dir> --windowed   # in a 640×480 window (dev)
```

It runs **fullscreen** by default (screensaver behavior); any key/click exits. Requires
stable Rust. `--data` also accepts the Internet Archive `.zip` directly (the run zip or the
installer); see [docs/INSTALL.md](docs/INSTALL.md).

### Options

Pass on the command line (override the file, for that run only) or edit the config file
(created on first use; find its path with `wilson /c`):

| Option | Values | Effect |
|---|---|---|
| `--windowed` | — | run in a window instead of fullscreen (`windowed=true`) |
| `--mute` | — | disable sound effects (`mute=true`) |
| `--speed <pct>` | `25`–`400` | animation speed, % of original (`speed=100`) |
| `--scale <mode>` | `fit`\|`stretch`\|`integer`\|`extend` | how the image fills the window; `extend` fills widescreen (`scale=fit`) |
| `--filter <mode>` | `nearest`\|`linear`\|`xbr`\|`xbrz` | upscaling filter (`filter=linear`) |
| `--dedither` | — | smooth the dithered sea/sky (default off = authentic look) (`dedither=false`) |
| `--daynight <mode>` | `original`\|`real24h` | day/night cycle: 8 h as in 1992, or 24 h by the clock (`daynight=original`) |

**Windows screensaver verbs:** `/s` (show), `/p <hwnd>` (preview embedded in the config
dialog's thumbnail — Windows only), `/c` (config — prints the options, the config file path
and the **stats**: sessions, total time and highest day reached).

## Knowledge base

All the reverse-engineering, the content catalog and the implementation plan are documented
in **[`docs/knowledge-base/`](docs/knowledge-base/README.md)**:

- [01 — History & Credits](docs/knowledge-base/01-historia-e-creditos.md)
- [02 — Content Bible](docs/knowledge-base/02-biblia-de-conteudo.md) *(every feature of the original)*
- [03 — Original Data & Formats](docs/knowledge-base/03-dados-originais-e-formatos.md)
- [04 — Scripting Engine: TTM/ADS Opcodes](docs/knowledge-base/04-engine-scripting-opcodes.md)
- [05 — Engine Architecture](docs/knowledge-base/05-arquitetura-do-engine.md)
- [06 — Reference Projects](docs/knowledge-base/06-projetos-de-referencia.md)
- [07 — Modern Port Plan](docs/knowledge-base/07-plano-do-port-moderno.md)

See also **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** for the data→pixels pipeline and how
to validate. *(The knowledge base is currently in Portuguese; English translation is in
progress.)*

## External references

Wilson Reborn was built by studying five independent, open-source reimplementations of the
original engine (details in [knowledge base 06](docs/knowledge-base/06-projetos-de-referencia.md)).
They are **not** vendored in this repository — clone the upstreams to compare:

- [jc_reborn](https://github.com/jno6809/jc_reborn) (C/SDL2) — the primary gameplay blueprint
- [ScummVM `dgds`](https://github.com/scummvm/scummvm) (C++) — the DGDS format authority (`engines/dgds`)
- [Johnny-Castaway-Open-Source / JCOS](https://github.com/nivs1978/Johnny-Castaway-Open-Source) (C#) — opcode dictionary
- [castaway](https://github.com/xesf/castaway) & [dgds-viewer](https://github.com/xesf/dgds-viewer) (JavaScript) — metadata & tooling

## Installation / packaging

Prebuilt binaries (Windows `wilson.scr` and Linux) are published on each version tag by the
release workflow. See **[docs/INSTALL.md](docs/INSTALL.md)** to install the Windows
screensaver, run on Linux/macOS, and cut releases.

> **Self-contained build (personal use):** if you already own the game, you can embed the
> data in the binary with `WILSON_EMBED_DATA=<dir> cargo build --release -p wilson --features
> embed-data` — a single file that runs without `--data`. The data is copyrighted and is
> **not** redistributed, so that build is never published in releases.

> **Run it in a browser (WASM):** the engine also compiles to WebAssembly — see
> **[`crates/wilson-web`](crates/wilson-web/README.md)**. Build it with
> `crates/wilson-web/build-web.sh` (or `scripts/build-embedded.sh --web`). You pick your own
> `RESOURCE.MAP`/`RESOURCE.001` in the page (read locally — nothing is uploaded), so no data is
> bundled (unlike the desktop `embed-data` build).

## Status

✅ **Complete engine in Rust + live window** (Johnny already runs on screen). Crates:

- `wilson-dgds` — DGDS formats: `RESOURCE.MAP/.001`, RLE/LZW, `.BMP/.SCR/.TTM/.ADS`, disassembler.
- `wilson-engine` — runtime: TTM/ADS interpreters, the director (63 scenes, 11-day cycle,
  holidays/tide/night), pathfinding, walk and island render; the `Show` integration.
- `wilson` — window app (winit + softbuffer) loading the **original files** (`--data` or
  auto-detection); sound, config/options, day persistence and stats.
- `wilson-saver` — the same engine exposed via FFI for a native macOS `.saver`.
- `wilson-web` — the engine compiled to WebAssembly to run in a browser (bring your own data).

Progress and decisions in
[`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md).
The focus is **full parity with the original data** (no recreated art).

## Contributing

See **[CONTRIBUTING.md](CONTRIBUTING.md)**. In short: every change must build and pass
`cargo fmt` / `clippy -D warnings` / the workspace tests; every bug fix comes with a
regression test; never commit game data.

## License

**GPL-3.0-or-later** — see [LICENSE](LICENSE). The original game data is **copyright
Sierra/Dynamix** and is never included in this repository.
