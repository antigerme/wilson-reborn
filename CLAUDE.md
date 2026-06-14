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
```
> Os dados originais (`RESOURCE.*`) são **copyright** e **não** ficam no repo. Os testes
> usam fixtures sintéticas — rodam sem os dados originais (essencial para o CI).

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
(seção "Status"). Resumo: **Fase 0 ✅ e Fase 1a ✅** — o crate `wilson-dgds`
decodifica `RESOURCE.MAP/.001`, RLE/LZW, chunks, paleta, `.BMP/.SCR/.TTM/.ADS` e o
`Archive` (30 testes, CI verde). **Próximo: Fase 1b** (bytecode TTM/ADS → instruções + VMs).
