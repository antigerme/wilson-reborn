# Wilson Reborn

Um clone **moderno, portável e melhorado** do clássico screensaver **Johnny Castaway**
(Sierra/Dynamix, 1992) — "o primeiro protetor de tela que conta uma história".

O objetivo é trazer de volta o Johnny com **paridade total** (todos os gags, eventos,
sequências narrativas, easter eggs, datas comemorativas e comportamentos do original),
rodando em **Windows e Linux** com **resoluções modernas** e melhorias opcionais, sem
perder nada do original.

## 📚 Base de Conhecimento

Toda a engenharia reversa, o catálogo de conteúdo e o plano de implementação estão
documentados em **[`docs/knowledge-base/`](docs/knowledge-base/README.md)**:

- [01 — História e Créditos](docs/knowledge-base/01-historia-e-creditos.md)
- [02 — Bíblia de Conteúdo](docs/knowledge-base/02-biblia-de-conteudo.md) *(todos os recursos do original)*
- [03 — Dados Originais e Formatos](docs/knowledge-base/03-dados-originais-e-formatos.md)
- [04 — Motor de Scripting: Opcodes TTM/ADS](docs/knowledge-base/04-engine-scripting-opcodes.md)
- [05 — Arquitetura do Engine](docs/knowledge-base/05-arquitetura-do-engine.md)
- [06 — Projetos de Referência](docs/knowledge-base/06-projetos-de-referencia.md)
- [07 — Plano do Port Moderno](docs/knowledge-base/07-plano-do-port-moderno.md)

## 📁 `repos/` — referências open-source

Cinco reimplementações independentes do motor original, usadas como referência (ver
[06](docs/knowledge-base/06-projetos-de-referencia.md)):
`jc_reborn` (C/SDL2), `dgds` (ScummVM/C++), `Johnny-Castaway-Open-Source`/JCOS (C#),
`castaway` e `dgds-viewer` (JavaScript).

## Como rodar

```bash
cargo run -p wilson                    # janela com o asset pack recriado embutido
cargo run -p wilson -- --data <dir>    # usando seus RESOURCE.MAP/RESOURCE.001 originais
```
Qualquer tecla/clique encerra (como um screensaver). Requer Rust estável.

## Status

✅ **Engine completo em Rust + janela ao vivo** (o Johnny já roda na tela). Crates:
- `wilson-dgds` — formatos DGDS: `RESOURCE.MAP/.001`, RLE/LZW, `.BMP/.SCR/.TTM/.ADS`, disassembler.
- `wilson-engine` — runtime: interpretadores TTM/ADS, diretor (63 cenas, ciclo de 11 dias,
  feriados/maré/noite), pathfinding, walk e render da ilha; integração `Show`.
- `wilson` — app de janela (winit + softbuffer) com asset pack **recriado** (copyright-free).

CI verde em **Ubuntu, Windows e Fedora**. Progresso e decisões em
[`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md).
Próximo: arte recriada melhor, som, e empacotamento `.scr`/instaladores.
