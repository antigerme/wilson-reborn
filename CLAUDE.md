# CLAUDE.md — Guia de continuidade (leia primeiro a cada sessão)

Este arquivo existe para **não perder memória entre sessões**. Se você é uma nova
instância do Claude Code, leia este arquivo e os links abaixo antes de agir.

## O projeto
**Wilson Reborn** = clone moderno, portável e melhorado do screensaver **Johnny
Castaway** (Sierra/Dynamix, 1992). Objetivo: **paridade total** com o original + rodar em
Windows/Linux com resoluções modernas e melhorias.

## Onde está o conhecimento
- **Base de conhecimento completa:** [`docs/knowledge-base/`](docs/knowledge-base/README.md)
  (história, bíblia de conteúdo, formatos, opcodes, arquitetura, plano do port).
- **Decisões e status atual:** [`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md)
- **Log cronológico:** [`docs/PROJECT-LOG.md`](docs/PROJECT-LOG.md)
- **Referências open-source** (5 reimplementações) em [`repos/`](repos/).

## Decisões já tomadas (NÃO reabrir sem o usuário pedir)
1. **Linguagem:** Rust (workspace Cargo em `crates/`).
2. **Assets:** híbrido — carregar os dados originais do usuário **e** mirar um pacote
   redistribuível recriado.
3. **Escopo:** incluir todas as melhorias possíveis (mas em incrementos 100% funcionais).
4. **Licença:** **GPL-3.0-or-later** (permite reusar jc_reborn/JCOS/ScummVM + MIT).

## Como construir e testar
```bash
cargo fmt --all -- --check      # formatação
cargo clippy --workspace --all-targets -- -D warnings   # lint (zero warnings)
cargo build --workspace
cargo test --workspace          # testes
cargo run -p wilson             # roda o app (assets recriados embutidos)
cargo run -p wilson -- --data <dir>   # roda com os RESOURCE.* originais do usuário
WILSON_DATA_DIR=<dir> cargo test -p wilson-dgds --test real_data -- --nocapture  # valida dados reais
```
> Assets originais para teste: `repos/dist.zip` (senha: `felicio`) e `repos/jc_reborn.msi`
> (extrair com `7z x`). Extraia para fora do repo (ex.: `/tmp`) — são **copyright**.
> Os dados originais (`RESOURCE.*`) são **copyright** e **não** ficam no repo. Os testes
> usam fixtures sintéticas — rodam sem os dados originais (essencial para o CI).
> O app `wilson` traz um **asset pack recriado** (copyright-free) e roda standalone.

## Regras de trabalho (combinadas com o usuário)
- **Sempre 100% → 100%:** cada incremento compila, passa lint e testes (local **e** CI).
- **CI do GitHub** roda em `ubuntu-latest`, `windows-latest` e `fedora-latest`
  (container `fedora:latest`) — `.github/workflows/ci.yml`.
  Se o CI falhar, **resolver**.
- **PRs:** acompanhar PRs (conflitos/CI) e resolver. O usuário faz squash merge e apaga a
  branch. Posso abrir PR quando a branch estiver madura. Trabalhar em **branch nova**
  por incremento (`claude/...`), nunca direto na `main`.
- **Documentar tudo** aqui, no PROJECT-LOG e na knowledge-base para preservar memória.

## Status atual
Ver [`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md)
(seção "Status"). Resumo: **Fases 0–1e ✅**. `wilson-dgds` decodifica
`RESOURCE.MAP/.001`, RLE/LZW, chunks, paleta, `.BMP/.SCR/.TTM/.ADS`, `Archive` e o
bytecode TTM/ADS → instruções. **`wilson-engine`** tem: TTM (`ttm_exec`/`TtmVm`),
escalonador **ADS** (`AdsVm`), o **diretor** (`story`: 63 cenas, ciclo 11 dias,
feriados/maré/noite/jangada), o **pathfinding** (`path`), a **walk animation**
(`walk`/`walk_data`), o **render da ilha** (`island`) e a **integração `Show`**
(diretor+ilha+walk+ADS → fluxo de frames). O app **`wilson`** (winit + softbuffer)
mostra o Johnny **na tela** com um **asset pack recriado** embutido (ou `--data` p/ os
originais). **Validado contra os dados REAIS** (`RESOURCE.001` autêntico → o Johnny
original renderiza; teste gated `wilson-dgds/tests/real_data.rs`). Escala 4:3 com
letterbox. **Engine completo + janela + validação real**, 75 testes, CI verde.
**Próximo: Fase 2d** — som, persistência do dia, arte recriada melhor, empacotamento `.scr`.
