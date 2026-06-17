# 06 — Projetos de Referência (comparativo, reúso e licenças)

> Avaliação dos 5 projetos em `repos/` como referência para o Wilson Reborn: o que cada
> um faz melhor, o que reaproveitar e as implicações de licença. Notas técnicas
> detalhadas de cada um estão em [`raw/`](raw/).

> **⚠️ Os 5 projetos NÃO são mais vendorizados em `repos/`** — foram removidos do
> repositório público (não redistribuir código de terceiros nem material copyright). Para
> consultá-los, clone os upstreams:
>
> - **jc_reborn** (jno6809): <https://github.com/jno6809/jc_reborn>
> - **JCOS** / Johnny-Castaway-Open-Source (nivs1978): <https://github.com/nivs1978/Johnny-Castaway-Open-Source>
> - **castaway** (xesf): <https://github.com/xesf/castaway>
> - **dgds-viewer** (xesf): <https://github.com/xesf/dgds-viewer>
> - **dgds** (ScummVM): <https://github.com/scummvm/scummvm> (`engines/dgds`)
>
> As referências `repos/...` neste documento e na KB apontam para os caminhos **desses**
> projetos.

---

## 1. Quadro comparativo

| Projeto | Linguagem/Stack | Licença | Estado | Força principal |
|---|---|---|---|---|
| **jc_reborn** | C99 + SDL2 | **GPLv3** | Jogável, "toda cena funciona" (c/ imprecisões) | **Melhor blueprint de gameplay**: VMs quase completas, walk, escalonador, ilha, dia/feriado |
| **dgds (ScummVM)** | C++ (ScummVM) | **GPLv2+** | WIP exploratório (RotD/HoC) | **Autoridade do formato DGDS**: chunks, RLE/LZW, fontes, som/MIDI, hash |
| **JCOS** | C# / WinForms (.NET) | **GPLv3** | WIP, Windows-only, sem ciclo de dias | **Melhor doc de formatos+opcodes** (nomes em inglês, operandos); extraiu os `.wav` |
| **castaway** | JS (ES Modules), Canvas | ver `LICENSE` | WIP web | **Metadados SCRANTIC** (nomes de cena, história/dia), roadmap de melhorias |
| **dgds-viewer** | JS + React + Electron | ver `LICENSE` | Visualizador multi-jogo | **Melhor tooling JS**: parsers defensivos, disassembler, UI de inspeção |

---

## 2. Avaliação individual

### 2.1 `jc_reborn` — a base de gameplay ⭐
**É a referência primária do Wilson Reborn.** Carrega os dados originais e reproduz o
comportamento interpretando TTM/ADS. Arquitetura limpa em 4 camadas (I/O, VM, backend,
lógica). Implementa o que nenhum outro tem junto: **walk entre spots, escalonador
aleatório de cenas, desenho da ilha/nuvens, ciclo de 11 dias e feriados**.

- **Reaproveitar:** semântica de opcodes (§[04](04-engine-scripting-opcodes.md)); as 3
  tabelas de dados (`story_data.h`, `walk_data.h`, `calcpath_data.h`); a lógica do
  diretor/escalonador; a lógica de dia/noite/maré/feriado.
- **Cuidados:** walk, escalonador, posicionamento da ilha e algumas ops de zona
  (`grSaveImage1`, `grSaveZone`) são **aproximações observacionais** (o autor admite); um
  disassembly do original as refinaria. Render re-blita tudo por frame (sem dirty-rect).
- **Licença:** GPLv3 — copiar código contamina; reusar os *insights* documentados (esta
  KB) não.

### 2.2 `dgds` (ScummVM) — a autoridade do formato ⭐
Engine ScummVM para a família DGDS. **Não cobre o JC** no `detection_tables.h` (lista só
Rise of the Dragon e Heart of China, que usam `VOLUME.RMF` em vez de `RESOURCE.MAP`), mas
o **formato de chunk, compressão, fontes, paleta e som são compartilhados**.

- **Reaproveitar:** a especificação rigorosa do container (bit de 0x80000000, prefixo de
  chunk packed), RLE/LZW exatos (incl. a nuance `_cacheBits`), recombinação de planos
  BIN/VGA, paleta `<<2`, e o `dgdsHash()` (se um dia suportar outros jogos DGDS).
- **Cuidados:** muitos opcodes ADS/TTM são **stubs** (`warning("Unimplemented")`) — para
  semântica de opcodes do JC, o `jc_reborn` é melhor. `VQT:` não é decodificado.
- **Licença:** GPLv2+ (ScummVM).

### 2.3 `JCOS` — o pioneiro e dicionário de opcodes
Primeira decodificação completa dos dados (2015). `Instruction.cs` tem um `ToString()`
que **soletra os operandos de ~60 opcodes** — a melhor referência de nomes/tuplas.
Também **extraiu os 24 `.wav`** que o jc_reborn reaproveita.

- **Reaproveitar:** o dicionário de opcodes; a gramática de ADS/TTM/BMP/SCR/PAL; os
  `.wav`; o exportador de disassembly via Excel (`Excel.cs`).
- **Cuidados:** WinForms **Windows-only**, caminho `C:\SIERRA\SCRANTIC` fixo, **sem
  lógica de dia/feriado** (seletor aleatório de 4 cenas), retângulos de debug por cima,
  paleta de 16 cores fixa em vez da `.PAL` parseada, 2 opcodes de sprite não
  implementados.
- **Licença:** GPLv3.

### 2.4 `castaway` — metadados e roadmap
Port web (Canvas). Único com a **camada de história/cenas do SCRANTIC**:
`metadata/scenes.mjs` (nomes descritivos das cenas ACTIVITY), `story.mjs` (contador de
dia), `palette.mjs`. Tem uma **roadmap de melhorias** muito alinhada aos seus objetivos
(ver [07](07-plano-do-port-moderno.md) §enhancements).

- **Reaproveitar:** as **descrições de cena** (GAG DIVES, NATIVE, GULL READING…); a
  roadmap; a abordagem de render em Canvas (se o alvo for web).
- **Cuidados:** `story.mjs` ainda escolhe cena **aleatória uniforme** (sem o schedule
  real de dias); RLE2 lança erro.

### 2.5 `dgds-viewer` — o melhor tooling JS
Versão mais elaborada do castaway, como **visualizador genérico** de recursos DGDS (todos
os 5 jogos), com React/Electron e um **disassembler ao vivo** (`ScriptCode.jsx`).
Interpretador mais robusto (`process.js`, dispatch O(1)).

- **Reaproveitar:** os **parsers** (limpos, sem dependências, `DataView`); o disassembler
  para **inspecionar/depurar assets** durante o desenvolvimento.
- **Cuidados:** o interpretador (`process.*`) é o ponto fraco (estado global mutável, não
  reentrante, muitos NOPs, scheduling ADS com bugs) — usar como **referência semântica**,
  reescrever do zero.

---

## 3. Estratégia de reúso recomendada

1. **Conhecimento, não código (default).** Esta KB captura os *insights* (formatos,
   opcodes, lógica) de forma independente de licença. Reimplementar a partir dela mantém o
   Wilson Reborn livre da contaminação GPL — útil se quiser escolher a licença.
2. **Parsers JS** (`dgds-viewer`) são os mais fáceis de portar se o alvo for
   web/TypeScript.
3. **Tabelas de dados** (`story_data.h`, `walk_data.h`, `calcpath_data.h` do jc_reborn):
   são **dados/fatos** reconstruídos. Reusá-los acelera muito; avaliar implicação de
   licença (dados vs código criativo) ou **re-extrair** do `SCRANTIC.SCR`.
4. **Disassembler** (`dgds-viewer` ou `jc_reborn dump`): indispensável para validar o
   port contra os dados reais.
5. **Validação cruzada:** rodar a mesma cena em ≥2 implementações e comparar — a melhor
   forma de resolver as divergências de opcode ([04](04-engine-scripting-opcodes.md) §4).

---

## 4. Matriz de licenças (resumo)

| Origem | Licença | Implicação se **copiar código** |
|---|---|---|
| jc_reborn, JCOS | GPLv3 | obra derivada deve ser GPLv3 |
| dgds (ScummVM) | GPLv2+ | obra derivada deve ser GPL |
| castaway, dgds-viewer | ver `LICENSE` no upstream | conferir antes de copiar |
| **Esta KB (docs)** | — | reimplementar a partir de fatos/documentação não cria obra derivada do código |

> **Os dados do jogo** (`RESOURCE.*`, sprites, sons) permanecem **copyright
> Sierra/Dynamix** — nenhum projeto os redistribui (ver
> [01 §nota legal](01-historia-e-creditos.md)). Decisão de licença e de assets do Wilson
> Reborn está em [07](07-plano-do-port-moderno.md).
