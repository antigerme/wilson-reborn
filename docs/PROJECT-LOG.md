# Project Log — Wilson Reborn

Log cronológico das decisões e entregas. Entradas mais recentes no topo.
(Para o estado consolidado, ver
[`knowledge-base/08-decisoes-e-status.md`](knowledge-base/08-decisoes-e-status.md).)

---

## 2026-06-14 — Fase 1e: diretor de história (crate `wilson-engine`)

**Branch `claude/engine-story-director`** (a partir da `main` pós-merge do PR #6).

Porte de `story.c` + `story_data.h` como **lógica pura testável** (data/hora/RNG
injetados):
- `rng` (extraído do `ads_vm`): `Rng` xorshift compartilhado.
- `story`: tabela das **63 cenas** (`STORY_SCENES`) com flags/spots/headings/dia;
  `pick_scene` (seleção ponderada por flags+dia), `holiday_for_date` (Halloween/
  S.Patrício/Natal/Ano Novo via MMDD), `is_night` (ciclo 8h), `raft_for_day`,
  `island_from_scene` (maré/posição aleatória/jangada/feriado). `Director` com
  `advance_day` (ciclo 1–11, avança por mudança de data real) e `plan_run` que
  produz um `StoryRun` (cena final + cadeia de 6–19 cenas ambiente com walk entre
  spots + estado da ilha), espelhando `storyPlay`.

Saída é um **plano** (`StoryRun`/`ScenePlay`) que uma camada futura alimenta ao
`AdsVm` (+ walk + render). **54 testes** (34 dgds + 20 engine), incl. os 11 beats de
dia conferidos contra a história, feriados, noite/jangada, clamp/wrap do dia e
invariantes do plano. Validado local: fmt, clippy `-D warnings`, build release, 54/54.

**Próximo:** Fase 1f — walk/pathfinding entre os 6 spots (porte de `walk.c`/`calcpath.c`
+ tabelas `walk_data.h`/`calcpath_data.h`); depois render da ilha; depois backend real.

---

## 2026-06-14 — Fase 1d: escalonador ADS multi-thread (crate `wilson-engine`)

**Branch `claude/engine-ads-scheduler`** (a partir da `main` pós-merge do PR #5).

Porte do `adsPlay`/`adsPlayChunk`/`adsLoad` (`ads.c`) — junta várias animações TTM
numa cena completa. Refatoração para um núcleo compartilhado:
- `ttm_exec`: `TtmSlot` (instruções+tags+sprites), `TtmThread` (estado+camada) e
  `run_frame()` — execução de uma thread por frame. `TtmVm` (Fase 1c) reescrito sobre
  ele (sem mudar a API/testes); fundo (`LOAD_SCREEN`) é global, sprites por slot.
- `ads_vm`: `AdsVm::next_frame()` faz **uma iteração** do escalonador cooperativo de
  timestep variável: roda threads com timer 0, compõe camadas, calcula `mini`,
  decrementa timers, e no pós-processamento aplica goto, decrementa `sceneTimer`
  (ADD_SCENE negativo = duração), re-arma `sceneIterations` (positivo = nº de vezes)
  ou encerra + dispara gatilhos `IF_LASTPLAYED`. `adsPlayChunk` com blocos
  RANDOM (peso) / OR / IF_NOT_RUNNING / PLAY_SCENE / END / GOSUB_TAG. RNG xorshift
  determinístico (testes reprodutíveis).

**45 testes** (34 dgds + 11 engine), incl. cena ADS fim-a-fim (ADD_SCENE→TTM→frame
composto→término) e bloco RANDOM escolhendo exatamente uma cena.
Validado local: fmt, clippy `-D warnings`, build release, 45/45 testes.

**Próximo:** Fase 1e — diretor (`story.c`: ciclo de 11 dias, seleção de cenas,
feriados/maré/noite), walk/pathfinding entre spots e desenho da ilha; depois, backend
de render real.

---

## 2026-06-14 — Fase 1c: interpretador TTM headless + `Surface` (novo crate `wilson-engine`)

**Branch `claude/engine-ttm-vm`** (a partir da `main` pós-merge do PR #4).

Primeiro crate de runtime. Executa **uma thread TTM** desenhando numa `Surface`
indexada (headless, sem janela/GPU) — núcleo da animação, testável de forma
determinística. Porte fiel de `ttm.c`/`graphics.c`:
- `surface`: framebuffer indexado + primitivas (pixel, linha/círculo Bresenham, rect
  com clip, blit com cor-chave + flip), composição de camadas e `to_rgba` (paleta).
  `TRANSPARENT = 0xFF` (sentinela; cor-chave magenta do original).
- `ttm_vm`: `TtmVm::step()` roda opcodes até `UPDATE` (frame) ou fim; resolve
  `LOAD_SCREEN`/`LOAD_IMAGE` via `Archive`; `DRAW_SPRITE x,y,frame(slot interno),slot`;
  coords assinadas + offset `dx/dy`; clip só afeta rect+sprite (como no original);
  `PLAY_SAMPLE` vira evento de som no frame; `PURGE`/fim → `Finished`.
- Pendentes para fases seguintes (no-op por ora, como o original já faz nos seus
  stubs): saved-zones (`COPY_ZONE_TO_BG`/`SAVE_ZONE`), e looping por `sceneTimer`
  (é responsabilidade do ADS).

**43 testes** (34 dgds + 9 engine), incl. fim-a-fim load→draw→update com
transparência e composição. Validado local: fmt, clippy `-D warnings`, build release,
43/43 testes.

**Próximo:** Fase 1d — escalonador ADS (até 10 threads TTM + composição de camadas +
encadeamento reativo/RANDOM), usando `decode_ads`.

---

## 2026-06-14 — Fase 1b: disassembler de bytecode TTM/ADS (crate `wilson-dgds`)

**Branch `claude/dgds-bytecode-decoder`** (a partir da `main` pós-merge do PR #3).

Decodifica os bytecodes (que a Fase 1a expôs como bytes) em **instruções tipadas**:
- `ttm`: `decode_ttm` / `TtmInstruction` / `TtmArgs` (`Words`/`Str`) + `ttm_opcode_name`.
  Regra: nibble baixo = nº de args; `0xF` = string NUL-terminada com padding par.
- `ads`: `decode_ads` / `AdsInstruction` + `ads_opcode_info` (nome + nº de args fixo).
  Opcodes fora da tabela = `:TAG` (0 args), como no disassembler de referência.
- Conveniências `Ttm::instructions()` / `Ads::instructions()`.

Espelha exatamente `repos/jc_reborn/dump.c` (dumpTtm/dumpAds). Args ficam como `u16`
crus (o sinal — ex.: arg3 de `ADD_SCENE` — é reinterpretado pelo futuro interpretador).
**34 testes** (era 30): args/strings TTM (padding par/ímpar), opcode desconhecido
consome args, opcodes/tag ADS e arg3 negativo.
Validado local: fmt, clippy `-D warnings`, build release, 34/34 testes.

**Próximo:** Fase 1c — interpretadores executáveis (precisam de uma abstração de
render/áudio; provável novo crate `wilson-engine`).

---

## 2026-06-14 — Fase 1a: parsers de recursos + Archive (crate `wilson-dgds`)

**Branch `claude/dgds-resource-parsers`** (a partir da `main` pós-merge do PR #2).

Completa a **camada de decodificação de recursos**, sobre as primitivas da Fase 0:
- `reader.cstr()` — string NUL-terminada de tamanho variável (espelha o `getString`
  do jc_reborn; tabelas RES/TAG são empacotadas, não campos fixos de 40 bytes).
- `pixels::decode_4bpp` — 4bpp → índices de paleta (nibble alto primeiro), compartilhado.
- `scr` — imagem de tela cheia (`SCR:`/`DIM:`/`BIN:`), decodificada para índices.
- `bmp` — folha de sprites (`BMP:`/`INF:`/`BIN:`): N imagens, cada uma decodificada.
- `ttm` — script de animação (`VER/PAG/TT3/TTI/TAG`): versão, páginas, **bytecode**
  descomprimido e tabela de tags.
- `ads` — script de sequência (`VER/ADS/RES/SCR/TAG`): versão, tabela **RES**
  (slot→`.TTM`), **bytecode** e tags.
- `archive` — carregador que liga `RESOURCE.MAP` + `RESOURCE.001`, decodifica cada
  recurso por tipo e ignora desconhecidos (`.VIN`).

Tudo fiel ao `jc_reborn` (`resource.c`, `graphics.c`, `utils.c`) — sem parser genérico
de chunk (cada tipo tem layout próprio). **30 testes** (era 20) com fixtures sintéticas.
Validado local: fmt, clippy `-D warnings`, build release, 30/30 testes.

**Próximo:** Fase 1b — decodificar o **bytecode TTM/ADS** em instruções (disassembler) e,
depois, os interpretadores executáveis.

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
