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
cargo run -p wilson                    # tela cheia, com o asset pack recriado embutido
cargo run -p wilson -- --windowed      # em janela 640×480 (útil no desenvolvimento)
cargo run -p wilson -- --data <dir>    # usando seus RESOURCE.MAP/RESOURCE.001 originais
```
Roda em **tela cheia** por padrão (comportamento de screensaver); qualquer tecla/clique
encerra. Requer Rust estável.

### Opções

Passe por linha de comando (vencem o arquivo, só nesta execução) ou edite o arquivo de
configuração (criado no 1º uso; veja o caminho com `wilson /c`):

| Opção | Valores | Efeito |
|---|---|---|
| `--windowed` | — | roda em janela em vez de tela cheia (`windowed=true`) |
| `--mute` | — | desliga os efeitos sonoros (`mute=true`) |
| `--speed <pct>` | `25`–`400` | velocidade da animação, % do original (`speed=100`) |
| `--scale <modo>` | `fit`\|`stretch`\|`integer` | como a imagem preenche a janela (`scale=fit`) |
| `--daynight <modo>` | `original`\|`real24h` | ciclo dia/noite: 8h como em 1992, ou 24h pelo relógio (`daynight=original`) |

**Verbos de screensaver do Windows:** `/s` (mostrar), `/p` (preview — ainda não embutido),
`/c` (configuração — imprime as opções, o caminho do arquivo e as **estatísticas**:
sessões, tempo total e maior dia alcançado).

## Instalação / empacotamento

Binários prontos (Windows `wilson.scr` e Linux) são publicados a cada tag de versão pelo
workflow de release. Veja **[`docs/INSTALL.md`](docs/INSTALL.md)** para instalar o
screensaver no Windows, rodar no Linux e publicar releases.

## Status

✅ **Engine completo em Rust + janela ao vivo** (o Johnny já roda na tela). Crates:
- `wilson-dgds` — formatos DGDS: `RESOURCE.MAP/.001`, RLE/LZW, `.BMP/.SCR/.TTM/.ADS`, disassembler.
- `wilson-engine` — runtime: interpretadores TTM/ADS, diretor (63 cenas, ciclo de 11 dias,
  feriados/maré/noite), pathfinding, walk e render da ilha; integração `Show`.
- `wilson` — app de janela (winit + softbuffer) com asset pack **recriado** (copyright-free).

CI verde em **Ubuntu, Windows e Fedora**. Progresso e decisões em
[`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md).
Próximo: arte recriada melhor, som, e empacotamento `.scr`/instaladores.
