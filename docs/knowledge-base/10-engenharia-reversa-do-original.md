<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 10 — Engenharia Reversa do Original (relatório de paridade)

> **Objetivo:** responder, com evidência, à pergunta *"estamos esquecendo algo?"* —
> comparando a implementação do Wilson Reborn diretamente contra o binário original
> (`SCRANTIC.EXE`/`.SCR`) e contra os dados (`RESOURCE.001`). É RE **legítima de uma cópia
> própria** para uma reimplementação interoperável (mesma base do ScummVM/jc_reborn/JCOS).
>
> **Método.** (1) Parse da estrutura NE (Win16) do `SCRANTIC.EXE`; (2) localização e
> verificação **byte-a-byte** das tabelas hardcoded contra o nosso código; (3) busca de
> constantes/imediatos da nossa lógica no binário; (4) histograma exaustivo de **todo**
> opcode TTM/ADS usado no `RESOURCE.001` real, cruzado com os executores; (5) inventário de
> recursos; (6) auditoria de conteúdo contra a [bíblia](02-biblia-de-conteudo.md) e a
> [auditoria de paridade](09-paridade-e-easter-eggs.md). Ferramenta de disassembly
> disponível: só `objdump` (16-bit) — disassembly profundo de toda a lógica não foi viável,
> então a lógica observacional é avaliada por **constantes + comportamento**, não instrução
> a instrução.

## 1. O binário original (estrutura NE)

`SCRANTIC.EXE` = `SCRANTIC.SCR` (um screensaver Windows 3.x é um `.exe` renomeado), formato
**NE (16-bit segmentado)**, 295 952 bytes. Módulo interno: `SCRNATIC`.

- **14 segmentos:** 13 de **código** + 1 de **dados** (#14, 18 368 bytes, file `0x17e00`).
  As tabelas hardcoded (que **não** vêm do `RESOURCE.001`) vivem no segmento de dados.
- **API importada:** `MMSYSTEM` (som), `GDI` (gráficos 2D), `KERNEL`, `USER`. Pontos de
  entrada SCRNSAVE padrão (`ScreenSaverProc`, `ScreenSaverConfigureDialog`, diálogos de
  senha do Win3.1). **Conclusão:** é um screensaver GDI+WAV comum — **nenhum subsistema
  escondido** (sem rede, sem hardware especial, sem MIDI dedicado). Nossa stack (softbuffer
  2D + `rodio` WAV) cobre a mesma superfície. Os diálogos de senha são um recurso morto do
  Win3.1 — não é lacuna.

## 2. Tabelas hardcoded — verificação contra o binário

| Tabela | Resultado | Confiança |
|---|---|---|
| **`walk_data`** (animação de caminhada) | **489/489 entradas BYTE-IDÊNTICAS** ao original | **Prova binária** ✅ |
| `calcpath` (adjacência de pathfinding) | **não existe** como tabela no EXE | Reconstrução (soft spot) ⚠️ |
| `story_data` (63 cenas) + lógica de feriado/maré/deriva/scheduling | constantes **não** localizáveis como imediatos | Port do jc_reborn; conteúdo bate com a bíblia ⚠️ |

### 2.1 `walk_data` — byte-perfeito ✅
Localizada no segmento de dados (file `0x188ea`). O struct original é **3 words por frame
(stride 6)**: `(sprite_word, x+1, y)`, com **`flip` empacotado no bit 15 do `sprite_word`**.
A nossa `WALK_DATA` (`[flip, x+1, y, sprite]`, herdada do jc_reborn — que a **extrai** do
binário, via `extract_walk_data.c`) reproduz as **489** entradas exatamente: `x+1`, `y`,
`sprite` e `flip` conferem em 100% das linhas. **Sem divergência.**

> Nota: numa primeira leitura agrupei os campos como `(x+1, y, sprite)` e *pareceu* haver
> uma defasagem de 1 frame no sprite. Era erro de agrupamento — com o layout correto
> (`sprite` **primeiro**), bate exatamente. Registrado para não reabrir.

### 2.2 `calcpath` — reconstrução, não verificável ⚠️
A nossa matriz de adjacência `WALK_MATRIX[7][6][6]` **não aparece** no binário em nenhuma
forma testada (bytes/words/transposta). Isso confirma o que o próprio jc_reborn admite: o
pathfinding foi **reconstruído por observação**, não extraído. O original computa as rotas
por outro mecanismo (ou armazena de forma muito diferente). **Não há prova de paridade
byte-a-byte;** é um modelo plausível. Resolver exigiria desassemblar a rotina `calcpath`.

### 2.3 `story_data` + lógica de história — port do jc_reborn ⚠️
As constantes da nossa lógica (faixas de feriado `mmdd` como `1028<mmdd<1101`; ranges de
deriva da ilha `-222+rnd(109)` etc.) **não foram localizadas como imediatos co-locados** no
código — só o offset `-272` (LEFT_ISLAND) aparece. Ou seja: o original expressa essas
comparações de outra forma (provavelmente `mês`/`dia` separados, não o `mmdd` que o jc_reborn
reformulou). **O comportamento bate com a bíblia**, mas as fronteiras exatas (ex.: Halloween
incluir ou não 28/out) são do jc_reborn, **não provadas no binário.** As 63 cenas e o mapa
dia→cena (os *day-beats*) são o arco documentado (teste `day_beats_match_the_story`).

## 3. Cobertura de opcodes (sobre o `RESOURCE.001` real)

**Tudo que os dados realmente usam é tratado** — nada é silenciosamente ignorado.

- **TTM: 30 opcodes distintos usados → 30 tratados.** Implementados (com efeito): PURGE,
  UPDATE, SET_DELAY, SET_BMP_SLOT, GOTO_TAG, SET_COLORS, TIMER, SET_CLIP_ZONE,
  COPY_ZONE_TO_BG (38×), DRAW_PIXEL/LINE/RECT/CIRCLE, **DRAW_SPRITE (12 546×)**,
  DRAW_SPRITE_FLIP (2 822×), CLEAR_SCREEN, **PLAY_SAMPLE (535×)**, LOAD_SCREEN, LOAD_IMAGE,
  RESTORE_ZONE, TAG/LOCAL_TAG. No-ops fiéis ao jc_reborn: SET_PALETTE_SLOT, SET_FRAME1,
  SAVE_IMAGE1, SAVE_ZONE, DRAW_SCREEN, LOAD_PALETTE, `TTM_UNKNOWN_1`, e **`0x0080`
  DRAW_BACKGROUND (190×)**.
- **ADS: 18 opcodes usados → 18 tratados** (+66 *tags*). Contagens de argumentos conferem
  (o stream nunca dessincroniza; os 10 ADS decodificam limpos). No-ops fiéis: `IF_UNKNOWN_1`,
  `UNKNOWN_6`, `FADE_OUT` (no bytecode).

## 4. Inventário de recursos — tudo parseia

179 entradas no mapa → **1 PAL, 116 BMP, 10 SCR, 41 TTM, 10 ADS, 1 pulada** (`FILES.VIN`,
não-recurso, ignorada também pelo jc_reborn). **0 referências `RES:` quebradas; 66/66 tags
de cena ADS constroem e tocam.** Inventário idêntico ao nosso (as 10 `.ADS` de
ACTIVITY…WALKSTUF; 42 nomes `.TTM` incluindo os easter eggs `GJGULIVR`, `GJLILIPU`,
`SHARK1`, `SJMSSGE`, `SBREAKUP`, `THEEND`).

## 5. Conteúdo vs a bíblia

- **Arco de 11 dias (Mary + Suzy):** presente e exercitado (long-run viu d1–d11); day-beats
  batem com `story.c`.
- **Easter eggs / cenas raras:** presentes e alcançáveis — THE END (dia 11), Gulliver
  (`GJGULIVR`, usa SAVE/RESTORE_ZONE), cargueiro gigante (`VISITOR#3`, usa COPY_ZONE_TO_BG),
  tubarão (`SHARK1`), nativos (`GJNAT1/3`), "Terminator" (`GJVIS5/6`). Gags como
  fantasma/bolas de prata/relógio real são sub-sequências dentro do bytecode (rodam quando
  o script roda).
- **23 efeitos sonoros:** confirmados (22 ids referenciados por TTM `PLAY_SAMPLE` + o cue de
  transição de dia `sound 0` do diretor). O id 17 existe como WAV mas **nenhum TTM o
  referencia — igual no original** (não é lacuna nossa).
- **4 feriados:** `HOLIDAY.BMP` tem **exatamente 4 sub-imagens** → os dados só conseguem
  desenhar 4 props (Halloween/StPatrick/Natal/Ano-Novo). **4 de Julho está genuinamente
  ausente dos dados originais** — a pendência da bíblia fica **resolvida: não há nada a
  reproduzir.**

## 6. Lacunas / o que podemos estar deixando passar

| Sev. | Item | Detalhe |
|---|---|---|
| **MÉDIA** | **Transições de cena sem fade/wipe** | O jc_reborn faz `grFadeOut()` (5 estilos de wipe) **entre cenas** e no intro, via o *scene-runner* em C (não no bytecode — por isso o no-op de `0xF010` está certo). Nosso `Show::go_next_scene` (`show.rs:296`) faz **corte seco**. Diferença **visual real** (não é perda de conteúdo). *Obs.: a evidência vem do jc_reborn (reimplementação); que o original de 1992 também fazia fade é muito provável, mas não localizei a rotina de fade no binário.* |
| BAIXA | Telas de intro/fim não usadas | `INTRO.SCR` nunca é exibida; `THEEND.SCR` só aparece como o TTM do dia 11, não como sequência de encerramento autônoma (como o jc_reborn). Diferença de apresentação. |
| BAIXA | `0x0080` DRAW_BACKGROUND = no-op **não documentado** | Tratado corretamente pelo catch-all (igual ao stub do jc_reborn, que diz "no-op; libera slots"), mas **190× em 36 arquivos** e **castaway/dgds-viewer/JCOS o tratam como redesenho de fundo**. Se algum dia um bug visual aparecer, é o ponto de divergência nº 1 a investigar (desassemblar o handler `0x0080` do original). |
| BAIXA | `0xA054` SAVE_ZONE = no-op | Enquanto o par `0xA064` RESTORE_ZONE é implementado. Fiel ao jc_reborn (lá também é quase-stub); usado 1× (GJGULIVR.TTM). Sem defeito visível conhecido. |
| (soft) | `calcpath`, constantes de feriado/deriva/scheduling | Não verificáveis no binário (§2.2/§2.3) — são a RE observacional do jc_reborn, fielmente portada. |

## 7. Veredito

- **Sobre os DADOS originais: alta confiança de que não falta nada.** 100% dos opcodes
  TTM (30/30) e ADS (18/18) usados são tratados; 179/179 recursos parseiam; 66/66 cenas
  constroem; arco de 11 dias, day-beats, easter eggs, 23 sons e os 4 feriados (= o máximo que
  os dados permitem) estão presentes e cobertos por teste. **A pendência do "4 de Julho" da
  bíblia está resolvida (ausente do original).**
- **Sobre a LÓGICA observacional:** somos tão fiéis quanto a melhor referência (jc_reborn) —
  `walk_data` é **byte-perfeita**; `calcpath` e as fronteiras de feriado/deriva são a
  reconstrução do jc_reborn, que portamos fielmente, mas **não dá para provar paridade
  byte-a-byte com o original** sem um disassembly completo (fora do alcance do `objdump`).
- **Melhorias de paridade acionáveis** (revisadas pelo disassembly, §9): o **intro**
  (recurso real com toggle `Introduction`, que não exibimos) e o caminho de áudio **MCI**
  (`mciSendCommand`). *(O "fade entre cenas" foi **rebaixado** — ver §9.3: vinha do jc_reborn,
  não confirmado no original.)*

## 8. Como reproduzir esta análise

Com uma cópia própria do original em `<dir>` (ver [INSTALL](../INSTALL.md)):
```bash
# tabelas + estrutura: scripts ad-hoc de parse NE (segmentos, segmento de dados) e
#   comparação byte-a-byte de WALK_DATA em <dir>/SCRANTIC.EXE, offset 0x188ea, stride 6.
# cobertura/inventário/conteúdo:
WILSON_DATA_DIR=<dir> cargo run -p wilson-engine --example audit
WILSON_DATA_DIR=<dir> cargo test -p wilson-engine real_data_long_run_invariants -- --nocapture
WILSON_DATA_DIR=<dir> cargo test -p wilson-dgds --test real_data -- --nocapture
```

---

## 9. Disassembly completo do binário (capstone) — direto do original

> Pass profundo a pedido do usuário e **sem depender do jc_reborn**. Ferramenta: disassembler
> recursivo próprio (**capstone** 16-bit) com as **relocações NE resolvidas** (mapas de
> ordinais das `.spec` do Wine) → cada chamada de API e `call` interno fica rotulado. **255
> funções, 25 732 instruções, ~75% do código** por descida recursiva (o resto é o CRT da
> Borland, *procs* de callback do Windows e tabelas de dados). A listagem crua é mantida
> **local** (é obra derivada do código copyright — não vai pro repo); aqui ficam só os fatos.

### 9.1 Arquitetura (confirmada no binário)
- **Gráficos:** composição em DCs off-screen (`CreateCompatibleDC`/`Bitmap`, `SelectObject`
  ×59, `BitBlt` ×19) + **`StretchBlt` ×11 = escala pra tela** (o original escala, como nós).
  Primitivas vetoriais GDI (`LineTo`/`MoveTo`/`Rectangle`/`Ellipse`/`CreatePen`) = os opcodes
  TTM de desenho. **NENHUMA API de paleta** (`CreatePalette`/`RealizePalette`/`AnimatePalette`
  ausentes) ⇒ **sem animação de paleta**; nossa abordagem (`.PAL` → RGB → blit) confere.
- **Som:** `sndPlaySound` (MMSYSTEM.2) pros WAVs (`WAVESFX%d`) — **e também `mciSendCommand`**
  (MCI) em `seg5:0085` (segundo caminho de áudio; não temos).
- **Loop/tempo:** `SetTimer(…, 50 ms, …)` bombeia `WM_TIMER` (0x0113), mas o avanço é **paceado
  por tempo real** (`GetCurrentTime`, `elapsed × rate[0x2e14]` em ponto-fixo /100000 via helper
  de 32 bits `seg1:0302`) ⇒ **não é 50 ms/quadro fixo** (é frame-rate-independente). O jc_reborn
  aproxima com 20 ms/tick fixo; **o rate exato (`[0x2e14]`, definido na init) não foi cravado** —
  é o único número de timing em aberto.
- **Config:** INI `[ScreenSaver.ScreenAntics]` em `SCRANTIC.INI`: `Sounds`, `Introduction`,
  `Password`/`PasswordProtection`, `CurrentMonth` (persistência).

### 9.2 Interpretador TTM (`seg12`) — opcodes lidos do binário
Despacho por **busca linear em tabelas de opcodes** (opcode em `[0x46da]`; interpretador de
**dois passes**, flag `[0x46d8]`; slots de bitmap em `[0x2638]`/`[0x263e]`). As próprias tabelas:
- família **C**, `seg12:0x00fc`: `c01f c02f c031 … c0f4 c102 cf01 cf11` (16 variantes; os dados
  só usam `c051`=PLAY_SAMPLE — tratamos o subconjunto usado);
- família **A**, `0x03bc`: `a002 a0a4 a104 … a5a7 a601 a704 af02 af1f af2f`;
- **A0xx zona**, `0x1900`: `a014 a024 … a054(SAVE) a064(RESTORE) a094 a0b5`;
- **baixos**, `0x12bd`: `0010 0020 0070 0080 0090 00c0 00e0 0110 0400`;
- alias: o interpretador **remapeia** `0x1301→0xc051` e `0x1311→0xc061`.

**`0x0080` (DRAW_BACKGROUND) — RESOLVIDO:** o handler (`seg12:0806`) **libera o handle GDI do
slot de bitmap atual** (`call seg6:1845` = wrapper de `DeleteObject`) e zera o slot —
**gerência de memória, ZERO saída visual.** Logo: o jc_reborn estava **certo** ("frees image
slots"); castaway/dgds-viewer/JCOS erram ao chamá-lo de "redesenho de fundo"; **e o nosso
no-op está correto vs o ORIGINAL.** *(A dúvida LOW da §6 fica resolvida — não é lacuna.)*

### 9.3 Correções aos itens que vinham do jc_reborn (não do original)
- **Fade/wipe entre cenas (era "MÉDIA") → REBAIXADO p/ NÃO-CONFIRMADO.** No binário **não há
  fade de paleta** (sem APIs de paleta). Um *wipe* via BitBlt é possível, mas **não foi
  localizado** no código analisado. A evidência de fade vinha do **jc_reborn** (reimplementação)
  — então **pode não ser lacuna**. Cravar exige analisar o caminho de transição de cena.
- **Intro (era "BAIXA") → CONFIRMADO como recurso real:** existe `INTRO.SCR` + a chave de
  config **`Introduction`** (liga/desliga). Não exibimos o intro ⇒ **lacuna real**,
  binário-confirmada.

### 9.4 Novos achados (só no binário)
- **`mciSendCommand`** — caminho de áudio MCI além do `sndPlaySound` (não reproduzido).
- **Relógio em tempo real** — formato `"%2d:%02d %cm"` ⇒ um gag mostra a hora real do PC.
- Família **C completa** (`c01f…c0f4`, 16 variantes de PLAY_SAMPLE) — tratamos só a usada.

### 9.5 Veredito pós-disassembly
A reengenharia direta do binário **fortaleceu** a confiança: confirmou a arquitetura e o
conjunto de opcodes, e **resolveu o `0x0080`** (nosso no-op está certo vs o original). Lacunas
reais binário-confirmadas: **intro** (recurso com toggle, não exibido) e o caminho **MCI**.
O **fade** foi **rebaixado** (não confirmado no original). O **rate de tempo exato** é o único
número em aberto. `calcpath` e as constantes de feriado/deriva seguem na lógica não-traçada
(fiéis ao jc_reborn, sem prova binária).
