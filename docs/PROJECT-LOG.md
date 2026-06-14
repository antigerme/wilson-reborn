# Project Log — Wilson Reborn

Log cronológico das decisões e entregas. Entradas mais recentes no topo.
(Para o estado consolidado, ver
[`knowledge-base/08-decisoes-e-status.md`](knowledge-base/08-decisoes-e-status.md).)

---

## 2026-06-14 — Fase 2d: polimento funcional — config + opções (tela cheia, escala, som, velocidade)

**Branch `claude/affectionate-gates-6oc4we`** (a partir da `main` pós-merge do PR #16).

O usuário pediu para **implementar todas as melhorias** (em incrementos 100%). Começo
pelo polimento funcional, que dá a base para os próximos toggles.

- **`crates/wilson/src/config.rs`** (novo): `Config` (windowed, mute, speed%, scale)
  lido de `config.txt` no diretório de estado do usuário (ao lado do `state.txt`;
  `state_dir` agora é `pub(crate)`), com defaults sensatos + flags de CLI que vencem o
  arquivo (sem persistir). Best-effort, nunca entra em pânico.
- **`scale.rs`**: três modos — **fit** (letterbox, padrão), **stretch** (preenche) e
  **integer** (múltiplo inteiro, pixels nítidos; cai p/ fit se a janela for menor);
  `ScaleMode` + dispatcher + refactor (`blit_scaled`).
- **App agora roda em TELA CHEIA por padrão** (comportamento de screensaver; `--windowed`
  p/ dev). `--mute` (não abre dispositivo de áudio), `--speed 25–400`, `--scale`.
  Verbo `/c` imprime as opções + caminho do arquivo; `/p` sai limpo; parser de verbos
  (`/c`,`-s`,`/p:HWND`) testável.
- **README** atualizado com a tabela de opções e os verbos de screensaver.

**100 testes** (32 wilson [+config/scale/verbos] + 35 dgds + 33 engine). Validado: fmt,
clippy `-D warnings` (com **e** sem a feature `audio`), `build --release`; `wilson /c`
imprime a config corretamente.

**Próximo:** ciclo dia-noite real de 24h (opcional, mantendo o de 8h do original);
depois empacotamento `.scr`/instaladores e mais melhorias.

---

## 2026-06-14 — Fase 2d (3/4): arte recriada melhor + props de feriado visíveis

**Branch `claude/affectionate-gates-6oc4we`** (a partir da `main` pós-merge do PR #15).

Terceiro passo da Fase 2d: o asset pack recriado (copyright-free, **desenhado em
código**) deixou de ser retângulos chapados e passou a evocar o original. **Validado
visualmente contra os dados REAIS** do usuário (renderizei o original como referência).

- **`crates/wilson/src/assets.rs`** reescrito com um mini-canvas de pixel-art (elipses,
  linhas, discos, dither ordenado de Bayer) e sprites procedurais:
  - **Céu + horizonte + oceano** (antes era oceano até o topo): céu ciano de dia / azul
    estrelado de noite, faixa de horizonte, gradiente de mar com espuma; à noite, lua +
    rastro de brilho na água.
  - **Ilha plana e dourada** com **anel de espuma**, duna sombreada e textura (antes um
    cilindro de areia); **palmeira** com tronco avermelhado segmentado e folhas grandes
    caídas; **nuvens** fofas (sem mais "buracos"); **ondas** em escamas suaves; **jangada**
    que cresce com o dia (logs amarrados); **props de feriado** (abóbora, pote, pinheiro,
    fogos); **Johnny** mais parecido com um náufrago (cabelo/barba, camisa rasgada,
    bermuda) e **ancorado na base** (16×64) para pisar na ilha.
- **Engine (`island.rs`):** o **prop de feriado agora é composto no cenário** (antes a
  `holiday_layer` existia mas nunca era desenhada → feriados invisíveis). Mantém a
  camada separada para quem quiser usá-la.
- Ferramenta de screenshot **gated** (`WILSON_DUMP=<dir>`, opcional `WILSON_REAL_DIR`)
  para render end-to-end de demo **ou** dos dados reais — no-op no CI.

**Validação com os dados reais** (`dist.zip`/`jc_reborn.msi` do usuário): teste gated
`real_data` ✅ (117 bmp, 10 scr, 41 ttm, 10 ads); render do original confere; com o fix,
o **pinheiro de Natal aparece** na ilha tanto no pack recriado quanto nos dados reais.

**85 testes** (17 wilson + 35 dgds + 33 engine). Validado local: fmt, clippy `-D
warnings` (com **e** sem a feature `audio`), `build --release`, todos verdes.

**Próximo (2d, 4/4):** empacotamento `.scr` (Windows screensaver) / instaladores.

---

## 2026-06-14 — Fase 2d (2/4): persistência do dia (arco de 11 dias entre sessões)

**Branch `claude/affectionate-gates-6oc4we`** (a partir da `main` pós-merge do PR #14).

Segundo passo da Fase 2d. Antes, o `Director` sempre começava no **dia 1** a cada
lançamento; agora o **arco de 11 dias continua de onde parou** entre sessões.

- **Engine:** novo acessor **`Show::day_state() -> (u8, i32)`** (current_day,
  stored_yday) para o host ler e persistir. (O `Director` já tinha a lógica de
  `advance_day`: incrementa quando o dia do calendário muda, com clamp 1–11.)
- **App:** novo módulo **`crates/wilson/src/state.rs`** (`DayState`) — carrega/grava
  `current_day`+`stored_yday` num arquivo de texto no diretório de estado do usuário
  (Windows `%APPDATA%\WilsonReborn`; senão XDG `$XDG_STATE_HOME` ou
  `~/.local/state/wilson-reborn`). **Zero deps** (resolve o diretório via env vars).
  **Best-effort:** arquivo ausente/ilegível ⇒ começa no dia 1, nunca entra em pânico.
- **`main.rs`:** no startup, `DayState::load()` → `Director::new(dia, yday)` (ou dia 1);
  a cada frame, `show.set_clock(clock::now())` (vira o dia à meia-noite mesmo numa
  sessão longa) e **salva quando o dia muda** (guardado por `last_saved`, ~1 escrita/sessão).

**83 testes** (15 wilson [+5: round-trip parse/serialize, save/load, rejeições] + 35
dgds + 33 engine [+1: dia avança e é observável via `day_state`]). Validado local: fmt,
clippy `-D warnings` (com **e** sem a feature `audio`), `build --release`, todos verdes.

**Próximo (2d, 3/4):** arte recriada melhor (asset pack copyright-free).

---

## 2026-06-14 — Fase 2d (1/4): som (`.wav`) via `rodio`

**Branch `claude/audio`** (a partir da `main` pós-merge do PR #13).

Primeiro passo da Fase 2d (ordem combinada: **som** → persistência do dia → arte
recriada → empacotamento `.scr`). O engine já emitia os ids de efeito por frame
(`Frame.sounds: Vec<u16>`); agora o app os **toca**.

- Novo módulo **`crates/wilson/src/audio.rs`** — um `Audio` que carrega `soundN.wav`
  (0–24) do diretório de dados (`--data`) e toca via `rodio` (`OutputStream`/`Sink`/
  `Decoder`, em background com `detach`). Os `.wav` são os efeitos originais
  (extraídos do `jc_reborn.msi`); **não** são redistribuídos.
- **Atrás de uma feature opcional `audio`** (ligada por padrão; `rodio` com
  `default-features=false, features=["wav"]`). **Degrada para silêncio** sem a feature,
  sem dispositivo de áudio, ou sem os arquivos — **nunca entra em pânico** (essencial p/
  o CI headless). `main.rs`: `for &id in &frame.sounds { audio.play(id); }`.
- **CI:** deps de áudio (ALSA) adicionadas no Linux — passo `apt-get libasound2-dev
  pkg-config` (Ubuntu) e `alsa-lib-devel pkgconf-pkg-config` no container Fedora.

**77 testes** (10 wilson [+2: filename, silêncio sem dispositivo] + 35 dgds + 32
engine). Validado local: fmt, clippy `-D warnings` (com **e** sem a feature `audio`),
`build --release`, todos verdes. **Rodar com som:** `cargo run -p wilson -- --data <dir>`.

**Próximo (2d, 2/4):** persistência do dia da história (arco de 11 dias entre sessões).

---

## 2026-06-14 — Fase 2c: validação com dados REAIS + escala 4:3

**Branch `claude/real-data`** (a partir da `main`, que o usuário atualizou com os assets
originais: `dist.zip` [senha: felicio] e `repos/jc_reborn.msi`).

Extraí os dados autênticos (md5 do `RESOURCE.001` confere) e **validei o engine de ponta
a ponta contra eles** — a lacuna que faltava:
- `Archive::parse` no `RESOURCE.001` real: 180 recursos (pal=1, bmp=117, scr=10, ttm=41,
  ads=10; `FILES.VIN` ignorado). **LZW + ~37 mil instruções TTM/ADS decodificadas sem
  erro.** Centenas de frames renderizados; **o Johnny original aparece corretamente**
  (screenshots enviados ao usuário).
- Capturado por um **teste de integração gated** `crates/wilson-dgds/tests/real_data.rs`
  (pulado se `WILSON_DATA_DIR` não estiver setado → CI passa sem dados copyright).
- **Polimento:** escala com **proporção 4:3 + letterbox** (`scale_rgba_to_argb_fit`) no
  app, em vez de esticar a imagem.

**75 testes** (8 wilson + 35 dgds [34 lib + 1 integração] + 32 engine). Validado local:
fmt, clippy `-D warnings`, build release, todos verdes; e o teste gated passa com os
dados reais.

**Próximo (2d):** som (`.wav`), persistência do dia da história, arte recriada melhor,
empacotamento `.scr`.

---

## 2026-06-14 — Fase 2b: app de janela `wilson` (o Johnny na tela!)

**Branch `claude/app-window`** (a partir da `main` pós-merge do PR #11).

Novo crate **`wilson`** (binário): janela ao vivo com **winit 0.29 + softbuffer 0.4**
(buffer de CPU; optou-se por `softbuffer` em vez de `pixels/wgpu` — mais leve, sem
stack de GPU, CI mais rápido). Roda o `Show`, faz `Frame.surface.to_rgba(paleta)` e
escala (nearest) para a janela; qualquer tecla/clique encerra (comportamento de
screensaver). Verbos de screensaver do Windows (`/s`,`/p`,`/c`) aceitos.

Decisão do usuário (assets): **pacote recriado** — então o app traz um **asset pack
procedural embutido** (copyright-free: oceano + ilha de areia com palmeira + figura que
caminha), semente do pacote redistribuível. `--data <dir>` carrega os `RESOURCE.*`
originais (loader `assets::load_real`). Relógio civil sem deps (`clock`, alg. de
Hinnant). Escala testável (`scale`).

CI: deps de GUI adicionadas ao job Fedora (`wayland-devel libxkbcommon-devel
libX11-devel`) por segurança (winit/softbuffer usam dlopen, mas garante o link).

**74 testes** (8 wilson + 34 dgds + 32 engine), incl. o asset pack recriado
renderizando algo além do oceano. Validado local: fmt, clippy `-D warnings`, build
release, 74/74. Janela não roda no CI (sem display) — só compila; testada por inspeção
+ build. **Rodar:** `cargo run -p wilson` (demo) ou `cargo run -p wilson -- --data <dir>`.

**Próximo (2c):** arte recriada melhor, som (`.wav`), persistência do dia, e
empacotamento `.scr`/instaladores.

---

## 2026-06-14 — Fase 2a: integração `Show` (crate `wilson-engine`)

**Branch `claude/engine-integration`** (a partir da `main` pós-merge do PR #10).

Amarra tudo num **gerador de frames** (`show`), espelhando o loop de `storyPlay`:
- `Show::next_frame()` planeja um run (Diretor → `StoryRun`), constrói a `Island`, e
  para cada cena: faz o Johnny **caminhar** (`Walker`, compondo o sprite sobre o fundo
  da ilha, com oclusão atrás da palmeira) e então toca a cena **ADS** (`AdsVm`) sobre
  o fundo da ilha; ao esgotar as cenas, planeja o próximo run. Relógio (`Clock`)
  injetado (testável). Recursos ausentes **pulam** a cena (degrada sem travar).
- Suporte: `AdsVm::set_background` (compor sobre a ilha) e `Island::offset`/`redraw_tree`.

**66 testes** (34 dgds + 32 engine): 400 frames cobrindo walks + cenas + troca de run,
e o caso de ADS ausente (frames em branco, sem travar).
Validado local: fmt, clippy `-D warnings`, build release, 66/66.

**Próximo:** Fase 2b — backend de render real (pixels/wgpu): `Frame.surface.to_rgba`
numa janela, modos de screensaver (`.scr` Win, fullscreen Linux), com os `RESOURCE.*`
do usuário. Aí o Johnny aparece na tela.

---

## 2026-06-14 — Fase 1h: render da ilha + Fase 1 completa (crate `wilson-engine`)

**Branch `claude/engine-island-render`** (a partir da `main` pós-merge do PR #9).

Porte de `island.c` (módulo `island`): `Island::build` pinta o cenário estático numa
`Surface` de fundo — tela `OCEAN0{0,1,2}`/`NIGHT`, jangada (`MRAFT`, posição muda com
maré), nuvens (`BACKGRND` 15–17, nº/vento aleatórios, espelhadas), ilha/tronco/folhas/
sombra (sprites 0/13/12/14), e na maré baixa praia+rocha (1/2). `animate_waves` faz a
animação cíclica das ondas (alta: 3 posições; baixa: 4) com os contadores do original.
Props de feriado (`HOLIDAY`) ficam numa camada própria. Tudo headless/testável.

**64 testes** (34 dgds + 30 engine): fundo+ilha+jangada nas posições certas, maré baixa
+ animação sem panic, e camada de feriado (árvore de Natal).
Validado local: fmt, clippy `-D warnings`, build release, 64/64.

### ✅ Fase 1 (engine) completa
Toda a lógica do engine está implementada e testada **headless**: dados →
descompressão → recursos → instruções → TTM → escalonador ADS → diretor (11 dias/
feriados) → pathfinding → walk → render da ilha. **Próximo (Fase 2):** uma camada de
integração que junta diretor+walk+ADS+ilha numa `Surface` por frame, e um **backend de
render real** (pixels/wgpu) + janela/screensaver — quando o Johnny aparece na tela.

---

## 2026-06-14 — Fase 1g: walk animation (crate `wilson-engine`)

**Branch `claude/engine-walk-animation`** (a partir da `main` pós-merge do PR #8).

Porte de `walk.c` + `walk_data.h`:
- `walk_data` (**gerado por script** `/tmp/gen_walk.py` a partir do C — 489 frames
  `[flip, x+1, y, sprite]` + tabelas de bookmarks/turns/headings). Os dados vêm do
  executável `SCRANTIC.SCR`, não do `RESOURCE.001`.
- `walk`: `Walker` (máquina de estados `walkInit`/`walkAnimate`) que usa `calc_path`
  + a tabela e produz um `WalkFrame` por chamada (virar → andar → chegar) até a
  chegada (delay 80). Expõe `flip/x/y/sprite/delay/behind_tree` (este último para o
  render redesenhar tronco/folhas ao cruzar D↔E). Rendering fica a cargo do chamador.

**61 testes** (34 dgds + 27 engine): caminhada entre todos os pares de spots
(termina, chega no spot certo, última pose com delay 80), giro no mesmo spot,
`behind_tree` na rota direta D↔E, e a regra de `turn_increment`.
Validado local: fmt, clippy `-D warnings`, build release, 61/61.

**Próximo:** Fase 1h — render da ilha (porte de `island.c`: fundo `OCEAN/NIGHT`,
jangada `MRAFT`, nuvens/ondas `BACKGRND`, props de feriado `HOLIDAY`). Depois o
backend de render real (Fase 2).

---

## 2026-06-14 — Fase 1f: pathfinding entre spots (crate `wilson-engine`)

**Branch `claude/engine-pathfinding`** (a partir da `main` pós-merge do PR #7).

Porte de `calcpath.c` + `calcpath_data.h` (módulo `path`): a **matriz de adjacência de
2ª ordem** `WALK_MATRIX[prev][cur][next]` (a rota permitida depende de onde Johnny
veio; o 1º salto usa a linha "de qualquer spot") e a enumeração DFS de caminhos
simples. `calc_paths(from,to)` lista todas as rotas; `calc_path(from,to,rng)` sorteia
uma. **57 testes** (34 dgds + 23 engine): um teste cobre **todos os 36 pares** de spots
(rota existe + começa/termina certo + simples + cada salto respeita a matriz),
validando a transcrição da tabela. Validado local: fmt, clippy `-D warnings`, build
release, 57/57.

**Próximo:** Fase 1g — walk animation (frames de `walk_data.h` + máquina de estados de
`walk.c`); depois render da ilha; depois backend de render real.

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
