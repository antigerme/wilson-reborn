# Base de Conhecimento — Wilson Reborn

Wilson Reborn é um **clone moderno e portável** do clássico screensaver **Johnny
Castaway** (Sierra/Dynamix, 1992). Esta base de conhecimento reúne, de forma completa e
não-superficial, **tudo** que é preciso saber para recriar o original e melhorá-lo: o
conteúdo (gags, história, easter eggs, feriados), o formato dos dados, o motor de
scripting e a arquitetura — destilados do site canônico de fãs e de **5 reimplementações
open-source** (não vendorizadas no repo; clonáveis nos upstreams — ver
[06](06-projetos-de-referencia.md)).

> **Como esta KB foi montada:** captura integral do site
> [johnny-castaway.com](https://johnny-castaway.com/) (todas as páginas e sublinks) +
> leitura profunda de **todos** os arquivos das 5 reimplementações (jc_reborn, dgds/ScummVM, JCOS,
> castaway, dgds-viewer), com validação cruzada entre as implementações.

---

## Documentos

| # | Documento | Conteúdo |
|---|---|---|
| 01 | [História e Créditos](01-historia-e-creditos.md) | Origem, criadores, marca *Screen Antics*, o motor DGDS, a comunidade e a linhagem open-source |
| 02 | [**Bíblia de Conteúdo**](02-biblia-de-conteudo.md) | **TODOS** os comportamentos, gags, personagens, visitantes, datas comemorativas, easter eggs, o arco de 11 dias, bugs e o mapa cena→comportamento. *(o registro "não perder nada")* |
| 03 | [Dados Originais e Formatos](03-dados-originais-e-formatos.md) | `RESOURCE.MAP/.001`, container de chunks DGDS, RLE/LZW, formatos `.ADS/.TTM/.SCR/.BMP/.PAL` |
| 04 | [Motor de Scripting: Opcodes](04-engine-scripting-opcodes.md) | Referência completa de opcodes **TTM** e **ADS** (consolidada das 4 implementações) |
| 05 | [Arquitetura do Engine](05-arquitetura-do-engine.md) | Loop principal, tick de 20 ms, diretor de história, walk/pathfinding, render em camadas, ilha, som |
| 06 | [Projetos de Referência](06-projetos-de-referencia.md) | Comparativo dos 5 repos, o que reusar, licenças |
| 07 | [**Plano do Port Moderno**](07-plano-do-port-moderno.md) | Stack recomendada, independência de resolução, empacotamento, **roadmap de melhorias**, plano faseado, decisões em aberto |
| 08 | [Decisões e Status](08-decisoes-e-status.md) | Estado consolidado: decisões firmes (ADR), processos e o roadmap por fases |
| 09 | [**Auditoria de Paridade e Easter Eggs**](09-paridade-e-easter-eggs.md) | Confronto bíblia × implementação: com os dados originais (`--data`/auto-detecção/`embed-data`) = **paridade total** *(o "pack recriado" foi removido no pivô de 2026-06-15; mantido só como histórico)* |
| 10 | [**Engenharia Reversa do Original**](10-engenharia-reversa-do-original.md) | Verificação **byte-a-byte** das tabelas vs o `SCRANTIC.EXE`, cobertura de opcodes/recursos sobre os dados reais, e o **relatório de lacunas de paridade** |

**Notas técnicas brutas** (dumps detalhados por repositório, com tabelas completas de
opcodes e referências file:line): [`raw/`](raw/).

---

## Resumo de 1 minuto

- **O que é:** "o primeiro screensaver que conta uma história" — Johnny, um náufrago numa
  ilha com um coqueiro, vive gags e uma **narrativa de 11 dias** (a sereia *Mary* e a moça
  da cidade *Suzy*) que **avança conforme a data real** do computador.
- **Como funciona:** o motor **DGDS/SCRANTIC** interpreta dois bytecodes — **TTM**
  (animação por cena) e **ADS** (sequenciamento) — sobre dados em `RESOURCE.001`. Um
  **diretor** (`story`) sorteia cenas, faz o Johnny **caminhar** entre 6 pontos da ilha, e
  aplica **dia/noite, maré e feriados**.
- **Referência-ouro:** **`jc_reborn`** (C/SDL2) para gameplay; **ScummVM `dgds`** (C++)
  para o formato; **JCOS** (C#) para o dicionário de opcodes; **castaway/dgds-viewer**
  (JS) para metadados e tooling.
- **Para o port:** mirror das 4 camadas (I/O → VMs → backend → lógica), tick de 20 ms,
  portar verbatim 3 tabelas que não vêm dos dados (`story/walk/calcpath`), e ler
  `RESOURCE.*` com RLE/LZW + chunks.
- **Decisões em aberto** (ver [07 §10](07-plano-do-port-moderno.md#10-decisões-para-confirmar-)):
  linguagem/stack, estratégia de assets, escopo do MVP, licença.

---

## Fontes
- Site canônico: https://johnny-castaway.com/ (todas as páginas).
- Reimplementações de referência (**não vendorizadas** no repo — clone os upstreams):
  `jc_reborn` (jno6809) <https://github.com/jno6809/jc_reborn>;
  `Johnny-Castaway-Open-Source`/JCOS (nivs1978) <https://github.com/nivs1978/Johnny-Castaway-Open-Source>;
  `castaway` & `dgds-viewer` (xesf) <https://github.com/xesf/castaway> · <https://github.com/xesf/dgds-viewer>;
  `dgds` (ScummVM) <https://github.com/scummvm/scummvm>.
- Wikipedia, Computer Gaming World, Dynamix Wiki, Sierra Chest.
