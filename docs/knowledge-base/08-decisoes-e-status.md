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
- `wilson-engine` — VMs TTM/ADS + diretor/story + walk + ilha + **integração (`Show`)**.
  **✅ engine headless completo** (de `RESOURCE.*` a um fluxo de frames compostos).
- `wilson` — app/janela (winit + **softbuffer**, CPU) + asset pack recriado + loader
  dos `RESOURCE.*`. **✅ janela ao vivo rodando.** (Optou-se por `softbuffer` em vez de
  `pixels/wgpu`: mais leve, sem stack de GPU, CI mais rápido — encaixa no engine, que já
  produz um buffer de CPU.)

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
| **1g** | **Walk animation (frames de `walk_data.h` + máquina de estados `Walker`)** | ✅ concluída (PR #9) |
| **1h** | **Render da ilha (fundo, jangada, nuvens, ondas, props de feriado)** | ✅ concluída (PR #10) — **Fase 1 (engine headless) completa** |
| **2a** | **Integração (`Show`): diretor + ilha + walk + ADS → fluxo de frames** | ✅ concluída (PR #11) |
| **2b** | **App `wilson`: janela ao vivo (winit + softbuffer) + asset pack recriado + loader `RESOURCE.*`** | ✅ concluída (PR #12) |
| **2c** | **Validação contra dados REAIS (teste gated) + escala 4:3 (letterbox)** | ✅ concluída — **engine renderiza o Johnny original** |
| 2d | Polir: som, persistência do dia, arte recriada melhor, empacotamento `.scr` | 🟡 **em curso** — ✅ **som** · ✅ **persistência do dia** · ✅ **arte recriada melhor** · ✅ **polimento funcional** (config `config.rs`: tela cheia, escala fit/stretch/integer, mute, velocidade); próximo: dia-noite 24h, empacotamento `.scr` |
| 3 | Empacotamento (Win/Linux/web/WASM) + assets → **paridade jogável** | ⬜ |
| 4 | Melhorias (HD, dia/noite 24h, config UI, estatísticas, etc.) | 🟡 **em curso** — o usuário pediu **todas as melhorias** (em incrementos): ✅ config/opções · ✅ **ciclo dia-noite 24h** (opção `daynight`, preservando o de 8h); próximo: empacotamento `.scr`, HD, estatísticas |

## Validação de dados reais ✅
Validado contra o `RESOURCE.001` **autêntico** (md5 `374e6d05…`): 180 recursos
(pal=1, bmp=117, scr=10, ttm=41, ads=10), **LZW + ~37 mil instruções TTM/ADS
decodificadas sem erro**, e centenas de frames renderizados (o Johnny original aparece
corretamente). Capturado por um **teste de integração gated** (pulado no CI, sem dados
copyright):
```sh
WILSON_DATA_DIR=/caminho/para/dist cargo test -p wilson-dgds --test real_data -- --nocapture
```
> Os dados originais e arquivos copyright (`RESOURCE.*`, `dist.zip`, `.msi`) **não** são
> redistribuídos pelo engine; o app traz um asset pack recriado e aceita `--data` para os
> dados do usuário.

### Histórico (antes da validação)
Os testes usavam apenas fixtures sintéticas; a validação byte-exata do LZW/parsers contra
um `RESOURCE.001` real estava planejada como teste de integração opcional via variável de
ambiente
apontando para os dados originais).
