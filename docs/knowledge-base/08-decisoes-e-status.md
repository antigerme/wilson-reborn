# 08 — Decisões e Status do Projeto

> Estado **consolidado** do projeto (decisões firmes + onde estamos). Atualizar a cada
> incremento. Para o histórico cronológico, ver [`../PROJECT-LOG.md`](../PROJECT-LOG.md).

## Decisões firmes (ADR resumido)

| # | Decisão | Escolha | Racional |
|---|---|---|---|
| 1 | **Linguagem/stack** | **Rust** (workspace Cargo) | binário único, cross-compile Win/Linux + WASM, seguro, ideal para processo long-running de screensaver |
| 2 | **Assets** | **Híbrido** | carrega `RESOURCE.*` originais do usuário **e** (meta) um pacote recriado redistribuível, para uma versão "pronta para distribuição" |
| 3 | **Escopo** | **Todas as melhorias** | mas entregues em **incrementos 100% funcionais** |
| 4 | **Licença** | **GPL-3.0-or-later** | única que permite reusar jc_reborn/JCOS (GPLv3) + ScummVM (GPLv2+) + castaway/dgds-viewer (MIT). Todos compatíveis ⇒ GPLv3 |

## Processos permanentes (combinados com o usuário)
- Sair sempre de um ponto **100% funcionando** para outro **100% funcionando**.
- **Testes/validações completos** + **CI do GitHub** (Ubuntu + Windows + Fedora). CI vermelho ⇒ resolver.
- **Acompanhar PRs** (conflitos e CI) e resolver. Usuário faz squash merge e apaga a branch;
  posso abrir PR quando a branch amadurecer. Sempre branch nova por incremento.
- **Documentar tudo** (knowledge-base, este arquivo e PROJECT-LOG) para não perder memória.

## Arquitetura-alvo (resumo — ver [07](07-plano-do-port-moderno.md))
Camadas: I/O de dados → VMs (TTM/ADS) → backend de render/áudio → lógica de jogo →
plataformas (`.scr` Win / Linux / standalone / web).

Crates planejados:
- `wilson-dgds` — formatos + descompressão + recursos. **(camada de recursos completa)**
- `wilson-engine` — VMs TTM/ADS + diretor/story + walk + ilha. **(TTM, ADS, diretor, pathfinding e walk animation prontos; render da ilha pendente)**
- `wilson-render` — trait de backend (pixels/wgpu/canvas). *(planejado)*
- `wilson` — binário/app + modos screensaver. *(planejado)*

## Status (roadmap)

| Fase | Descrição | Estado |
|---|---|---|
| KB | Base de conhecimento | ✅ concluída (merged) |
| **0** | **Camada de dados** (`RESOURCE.*`, RLE/LZW, chunks, PAL) | ✅ concluída (PR #2) |
| **1a** | **Parsers `.BMP/.SCR/.TTM/.ADS` + `Archive`** | ✅ concluída (PR #3) |
| **1b** | **Decodificar bytecode TTM/ADS → instruções (disassembler)** | ✅ concluída (PR #4) |
| **1c** | **Interpretador TTM executável (headless, 1 thread) + `Surface`** | ✅ concluída (PR #5) |
| **1d** | **Escalonador ADS (multi-thread + composição + RANDOM/gatilhos)** | ✅ concluída (PR #6) |
| **1e** | **Diretor (story 11 dias, seleção, estado da ilha: maré/noite/jangada/feriado)** | ✅ concluída (PR #7) |
| **1f** | **Pathfinding entre os spots (matriz de adjacência 2ª ordem + rotas)** | ✅ concluída (PR #8) |
| **1g** | **Walk animation (frames de `walk_data.h` + máquina de estados `Walker`)** | ✅ concluída |
| 1h | Render da ilha (fundo, jangada, nuvens, ondas, props de feriado) | 🟡 **próximo** |
| 2 | Backend de render real (pixels/wgpu) + janela/screensaver | ⬜ |
| 3 | Empacotamento (`.scr` Win, Linux, web/WASM) → **paridade jogável** | ⬜ |
| 4 | Melhorias (HD, dia/noite 24h, config UI, estatísticas, etc.) | ⬜ |

## Validação de dados reais (pendente)
Os testes usam fixtures sintéticas (o CI não pode ter os dados copyright). **A validação
byte-exata do LZW/parsers contra um `RESOURCE.001` real** deve ser feita localmente por
quem tiver o arquivo (planejado: teste de integração opcional via variável de ambiente
apontando para os dados originais).
