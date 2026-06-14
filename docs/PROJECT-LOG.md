# Project Log — Wilson Reborn

Log cronológico das decisões e entregas. Entradas mais recentes no topo.
(Para o estado consolidado, ver
[`knowledge-base/08-decisoes-e-status.md`](knowledge-base/08-decisoes-e-status.md).)

---

## 2026-06-14 — Fase 0: camada de dados (crate `wilson-dgds`)

**Contexto:** decisões confirmadas pelo usuário — Rust, assets híbridos, todas as
melhorias, licença GPLv3. Início da implementação em incrementos 100% funcionais.

**Entregue neste incremento (branch `claude/engine-foundation`):**
- Workspace Cargo (`Cargo.toml`) + crate **`wilson-dgds`** (zero dependências externas,
  `#![forbid(unsafe_code)]`).
- **Camada de dados (Fase 0):**
  - `reader.rs` — cursor little-endian com checagem de limites.
  - `decompress.rs` — **RLE** e **LZW** (porte fiel de `repos/jc_reborn/uncompress.c`),
    + método 0 (none). LZW: 9→12 bits, LSB-first, code 256 = clear.
  - `chunk.rs` — header de chunk DGDS (tag `XXX:`, bit de container `0x80000000`) +
    leitura de bloco "packed".
  - `resource.rs` — parser do índice `RESOURCE.MAP` (formato JC: length+offset) e do
    cabeçalho de entrada em `RESOURCE.001`.
  - `pal.rs` — parser de paleta (`PAL:`/`VGA:`, 6-bit→8-bit).
- **Testes** unitários com fixtures sintéticas (rodam sem dados originais): round-trip
  LZW (incl. cruzamento 9→10 bits), vetores RLE, índice de recursos, paleta, chunks.
- **CI** (`.github/workflows/ci.yml`): fmt + clippy `-D warnings` + build + test em
  Ubuntu, Windows e Fedora (container `fedora:latest` em host Ubuntu, toolchain via rustup).
- **Licença** GPL-3.0-or-later (`LICENSE`).
- **Memória/continuidade:** `CLAUDE.md`, este log, e
  `knowledge-base/08-decisoes-e-status.md`.

**Validação local:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` — todos
verdes antes do push. (Ver knowledge-base/08 para o resultado registrado.)

**Próximo:** Fase 1 — parsers de `.BMP`/`.SCR`/`.TTM`/`.ADS` (container + tabelas RES/TAG)
e os interpretadores TTM/ADS.

---

## 2026-06-14 — Base de conhecimento (PR #1, merged)

Captura integral de https://johnny-castaway.com/ e leitura profunda dos 5 projetos em
`repos/`. Criada a `docs/knowledge-base/` (8 documentos + notas brutas em `raw/`).
Merged na `main` via squash.
