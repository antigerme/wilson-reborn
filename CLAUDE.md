# CLAUDE.md — Guia de continuidade (leia primeiro a cada sessão)

Este arquivo existe para **não perder memória entre sessões**. Se você é uma nova
instância do Claude Code, leia este arquivo e os links abaixo antes de agir.

## O projeto
**Wilson Reborn** = clone moderno, portável e melhorado do screensaver **Johnny
Castaway** (Sierra/Dynamix, 1992). Objetivo: **paridade total** com o original + rodar em
Windows/Linux/macOS com resoluções modernas e melhorias.

## Onde está o conhecimento
- **Base de conhecimento completa:** [`docs/knowledge-base/`](docs/knowledge-base/README.md)
  (história, bíblia de conteúdo, formatos, opcodes, arquitetura, plano do port).
- **Decisões e status atual:** [`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md)
- **Arquitetura (mapa do pipeline + como validar):** [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- **Log cronológico:** [`docs/PROJECT-LOG.md`](docs/PROJECT-LOG.md)
- **Referências open-source** (5 reimplementações: jc_reborn, JCOS, castaway, dgds-viewer,
  ScummVM/dgds) — **não vendorizadas** no repo (removidas em 2026-06-17); URLs em
  [`docs/knowledge-base/06-projetos-de-referencia.md`](docs/knowledge-base/06-projetos-de-referencia.md).

## Decisões já tomadas (NÃO reabrir sem o usuário pedir)
1. **Linguagem:** Rust (workspace Cargo em `crates/`).
2. **Assets:** **100% originais** — usar **apenas** os arquivos originais do usuário
   (`RESOURCE.MAP`/`RESOURCE.001`), via `--data` ou auto-detecção. **Sem pack recriado**
   (o pack de arte recriada foi **removido em 2026-06-15** — não atingiu a qualidade
   desejada; o foco é paridade total com os dados originais).
3. **Escopo:** incluir todas as melhorias possíveis (mas em incrementos 100% funcionais).
4. **Licença:** **GPL-3.0-or-later** (permite reusar jc_reborn/JCOS/ScummVM; demais
   conforme o LICENSE de cada upstream).

## Como construir e testar
```bash
cargo fmt --all -- --check      # formatação
cargo clippy --workspace --all-targets -- -D warnings   # lint (zero warnings)
cargo build --workspace
cargo test --workspace          # testes
cargo run -p wilson -- --data <dir>   # roda com os RESOURCE.* originais do usuário
WILSON_DATA_DIR=<dir> cargo test -p wilson-dgds --test real_data -- --nocapture  # valida dados reais
```
**Auto-validação (em vez de caçar erro a olho — ver [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)):**
```bash
# invariantes automáticos (no CI, sem dados): nunca paniqueia, frame 640x480, anima, ritmo humano
cargo test -p wilson-engine engine_run_stays_live_and_paced
# invariantes profundos (com dados): + 100% opaco (sem "água magenta") + dia avança
WILSON_DATA_DIR=<dir> cargo test -p wilson-engine real_data_long_run_invariants -- --nocapture
# revisão visual: renderiza a run em quadros p/ virar mosaico/mp4 com ffmpeg
cargo run -p wilson-engine --example render_run -- <dir> /tmp/out 27000 225 1
```
> Assets originais para teste: use sua **própria cópia** (`--data <dir-ou-zip>`) ou os zips
> do Internet Archive (abaixo). **Não ficam no repo** — `repos/` (que tinha o antigo
> `dist.zip`, protegido por senha) foi removido em 2026-06-17; os dados são **copyright**.
> **Fonte legal/pública dos originais:** o screensaver está preservado no Internet Archive
> — <https://archive.org/details/johnny-castaway-screensaver> (`scrantic-run.zip` traz
> `RESOURCE.MAP` + `RESOURCE.001` + `SCRANTIC.EXE`). Validado: o engine decodifica e roda
> essa cópia também (≠ md5 da cópia antiga; 116 bmp/10 scr/41 ttm/10 ads; invariantes +
> holidays OK). Documentado em `docs/INSTALL.md`.
> **Som dos originais:** os 23 efeitos ficam embutidos como WAVs **dentro do `SCRANTIC.EXE`**
> (não em `RESOURCE.001`); `wilson_dgds::sounds_from_scrantic_exe` os extrai (mapeados por
> tamanho do `data`, verificado byte-a-byte vs os `soundN.wav` do JCOS/jc_reborn). O app e o
> build `embed-data` caem nisso quando não há `soundN.wav`. **jc_reborn/JCOS NÃO são o
> original** — são reimplementações; os `soundN.wav` deles são os mesmos PCM re-embalados.
> **`--data` aceita pasta OU `.zip`** (o `scrantic-run.zip` e o `scrantic-installer.zip`),
> auto-detectados no cwd / ao lado do exe (`assets.rs::resolve_data_dir`, dep `zip`). O
> instalador usa compressão **PKWARE DCL** num wrapper Dynamix (magic `65 5d 13 8c`);
> `wilson_dgds::decompress_installer` (porte do `blast.c`) descomprime `RESOURCE.00$`→`.001`
> e `SCRANTIC.SC$`→`.SCR` (verificado byte-a-byte). `build.rs` faz o mesmo no embed.
> Os dados originais (`RESOURCE.*`) são **copyright** e **não** ficam no repo. Os testes
> usam fixtures sintéticas — rodam sem os dados originais (essencial para o CI).
> O app `wilson` **exige** os dados originais (`--data <dir-ou-zip>` ou auto-detecção no
> diretório atual / ao lado do executável); sem eles, explica o que falta e sai.

## Regras de trabalho (combinadas com o usuário)
- **Sempre 100% → 100%:** cada incremento compila, passa lint e testes (local **e** CI).
- **⚠️ TESTAR E VALIDAR DE VERDADE O ENTREGÁVEL — não só o CI — em TODAS as superfícies afetadas**
  (desktop, **Web/WASM**, embedded, `.scr` Windows, macOS), **antes** de dizer "pronto" ou abrir PR.
  CI verde ≠ funciona pro usuário. Re-verificar **comportamentos já corrigidos** (guardar contra
  regressão). Quando algo **não** der pra testar aqui (ex.: áudio do navegador exige um browser
  real; `.scr` exige Windows), **dizer isso explicitamente** e validar o máximo possível por outro
  meio (ex.: **smoke test em Node** do wasm: `wasm-bindgen --target nodejs` + checar
  `has_sound()`/`take_sounds()`/`sound_wav()`; cross-compile `windows-gnu`). Nunca declarar uma
  feature "funcionando" sem evidência. (Pedido enfático do usuário, 2026-06-18 — regressão do som
  no web.) (Nota: navegadores **bloqueiam áudio até o 1º gesto** do usuário — não é bug; a página
  inicia o som no primeiro clique/tecla/toque e a UI mostra esse estado "aguardando".)
- **⚠️ Web/WASM: validar num Chrome de verdade** com o harness Playwright em
  [`crates/wilson-web/e2e`](crates/wilson-web/e2e/README.md) (`node run.mjs`) — dirige a página
  como usuário. Build **embedded** (`WILSON_EMBED_DATA=… ../build-web.sh`) ⇒ teste **completo**
  (renderiza + **som toca de verdade** após o gesto, `?seed=0&speed=400` determinístico); build
  bring-your-own ⇒ **smoke** (carrega + wasm inicia + sem erros JS) — o smoke roda no **CI** (job
  `web-e2e`), o completo é local (precisa dos dados copyright). (Pedido do usuário, 2026-06-18.)
- **⚠️ Todo bug encontrado entra com teste de regressão que FALHA sem o fix** (verificar que
  falha antes de aplicar a correção) — para nunca reverter algo já corrigido. Vale também
  para invariantes de workflow/CI (ex.: lint do `release.yml`). (Pedido do usuário,
  2026-06-15.)
- **CI do GitHub** roda em `ubuntu-latest`, `windows-latest`, `fedora-latest`
  (container `fedora:latest`) e `macos-latest` — `.github/workflows/ci.yml`.
  Se o CI falhar, **resolver**.
- **⚠️ REGRA PERMANENTE — acompanhar SEMPRE o GitHub até o fim:** para todo PR que eu
  abrir/tocar, **seguir e resolver** PR + CI + reviews + conflitos + **merge** **até o PR
  estar MERGED ou CLOSED**. Inscrever no PR (`subscribe_pr_activity`); tratar cada evento
  (CI vermelho → diagnosticar e corrigir; review → responder/aplicar; conflito → rebase).
  Webhook **não** entrega sucesso de CI nem push/merge — então **re-checar** ativamente
  (ex.: pingar a API; sem `send_later`, usar espera em background) e **confirmar o verde**.
  Não considerar a tarefa encerrada enquanto houver PR pendente. (Pedido explícito do
  usuário, 2026-06-15.)
- **PRs:** o usuário faz squash merge e apaga a branch. Posso abrir PR quando a branch
  estiver madura. Trabalhar em **branch nova** por incremento (`claude/...`), nunca direto
  na `main`. **Releases:** o push de tag `v*` é bloqueado neste ambiente (403) — guiar o
  usuário a dar o `git tag -a vX.Y.Z -m … && git push origin vX.Y.Z` (a tag tem que ser
  **criada** antes de empurrar) e depois **acompanhar o `release.yml`** e conferir os artefatos.
  Artefatos da release: `wilson.scr`/`.exe` (Windows), binários Linux/macOS, `.saver` (macOS), e
  o **`wilson-web.zip`** (bundle Web/WASM **traga-seus-dados** — sem dados, copyright-safe; o
  embutido NÃO entra na release). ⚠️ `release.yml` **e `pages.yml`** **não podem** conter
  `RESOURCE.`/`embed-data`/`.wav`/`dist.zip` (lint `public_artifacts_do_not_ship_game_data`).
- **GitHub Pages** (`pages.yml`): a página traga-seus-dados é publicada **ao vivo** em
  <https://antigerme.github.io/wilson-reborn/> a cada push na `main` (deploy só fora de PR; build
  roda no PR). **Setup único do usuário:** Settings → Pages → Source = **GitHub Actions** (o 1º
  deploy falha até ligar). Nada hospedado é copyright (sem `WILSON_EMBED_DATA`).
- **Documentar tudo** aqui, no PROJECT-LOG e na knowledge-base para preservar memória.

## Status atual
Ver [`docs/knowledge-base/08-decisoes-e-status.md`](docs/knowledge-base/08-decisoes-e-status.md)
(seção "Status"). Resumo: **Fases 0–1e ✅**. `wilson-dgds` decodifica
`RESOURCE.MAP/.001`, RLE/LZW, chunks, paleta, `.BMP/.SCR/.TTM/.ADS`, `Archive` e o
bytecode TTM/ADS → instruções. **`wilson-engine`** tem: TTM (`ttm_exec`/`TtmVm`),
escalonador **ADS** (`AdsVm`), o **diretor** (`story`: 63 cenas, ciclo 11 dias,
feriados/maré/noite/jangada), o **pathfinding** (`path`), a **walk animation**
(`walk`/`walk_data`), o **render da ilha** (`island`) e a **integração `Show`**
(diretor+ilha+walk+ADS → fluxo de frames). O app **`wilson`** (winit + softbuffer)
mostra o Johnny **na tela** carregando os **arquivos originais** (`--data <dir>` ou
auto-detecção no diretório atual / ao lado do executável; sem dados, explica e sai).
**Validado contra os dados REAIS** (`RESOURCE.001` autêntico → o Johnny
original renderiza; teste gated `wilson-dgds/tests/real_data.rs`). Escala 4:3 com
letterbox. **Som** (`audio.rs`): toca `soundN.wav` via `rodio` (feature opcional
`audio`, ligada por padrão; degrada para silêncio sem dispositivo/arquivos), os efeitos
vêm com `--data`. Os 23 `soundN.wav` originais decodificam (verificado); efeitos in-scene
via TTM `0xC051`; **cue de transição de dia** (`sound 0`) emitido nas cenas de day-beat
(`Show::pending_sound`, igual ao `soundPlay(0)` do jc_reborn). **Persistência do dia** (`state.rs` + `Show::day_state`): o arco de 11
dias continua entre sessões (grava `current_day`/`stored_yday` no diretório de estado do
usuário; zero deps; best-effort). **Props de feriado** compostos **por cima** (Show
`overlay_holiday`, igual ao `grUpdateDisplay` do jc_reborn) ⇒ aparecem com `--data`.
**Polimento funcional** (`config.rs`): opções via `config.txt` +
flags de CLI — **tela cheia por padrão** (`--windowed`), escala fit/stretch/integer/extend
(`scale.rs`), `--mute`, `--speed 25–400`; verbo `/c` imprime a config. **Engine completo
+ janela + validação real + som + persistência + config**. **Ciclo
dia-noite**: `DayNight {Original 8h, Real24h}` no `story.rs` (opção `daynight`, padrão
original), aplicado via `Director::with_daynight`. **Empacotamento**: `release.yml` gera
`wilson.scr` (Windows) + binário Linux em tag `v*`/dispatch (artefatos + GitHub Release);
instalação em `docs/INSTALL.md`. **Estatísticas** (`stats.rs`): sessões, tempo total e
maior dia, persistidas em `stats.txt` e exibidas no `/c`. **Auditoria de paridade**
(knowledge-base [09](docs/knowledge-base/09-paridade-e-easter-eggs.md)): com `--data` o
engine roda os scripts originais ⇒ **paridade total de conteúdo** (validada). **Cobertura
100% de opcodes** (auditado nos dados reais): camada de **zonas salvas**
(`COPY_ZONE_TO_BG`/`RESTORE_ZONE`, `Surface::blit_zone`; composta fundo→zonas→threads) p/
o gag do cargueiro gigante; os demais no-ops batem com o `jc_reborn`.
**Pivô 2026-06-15:** o **pack de arte recriada foi removido** (não atingiu a qualidade
desejada); o app agora usa **100% os arquivos originais** — `assets.rs` ficou só com
`load`/`find_data_dir` e `main.rs` exige `--data`/auto-detecção. **Build autossuficiente**
(feature `embed-data` + `build.rs` + `embedded.rs`): `WILSON_EMBED_DATA=<dir> cargo build
--release --features embed-data` embute `RESOURCE.*` + `soundN.wav` no binário (lido só em
tempo de compilação, **nunca** versionado; uso pessoal por causa do copyright) ⇒ roda sem
`--data`. **CI verde.**
**Engenharia reversa do original (2026-06-18):** RE completa do `SCRANTIC.EXE` (NE/Win16)
documentada em [`docs/knowledge-base/10`](docs/knowledge-base/10-engenharia-reversa-do-original.md),
com **ferramentas reprodutíveis** salvas em
[`docs/reverse-engineering/`](docs/reverse-engineering/README.md) (`ne.py` + `disasm.py`; o
binário copyright e a listagem gerada ficam **locais**, fora do repo). Conclusões: paridade
alta sobre os dados; `walk_data` byte-idêntico; **`0x0080` e o áudio `MCI` resolvidos como
no-op/código-morto** (não são lacunas); a única lacuna real — o **intro** (`INTRO.SCR` +
opção `Introduction`) — foi **implementada** (`Show::enable_intro`, config `intro` padrão
ligado, flag `--no-intro`). A **knowledge-base foi traduzida para inglês** (11 docs).
**Release v0.2.0 (2026-06-18):** primeira release pública empacotada (tag `v*` → `release.yml`
publica `wilson.scr`/`.exe` Windows + binários Linux/macOS + `.saver`). Processo: **bumpar a
versão e mergear ANTES de taguear** (a v0.2.0 precisou de re-corte por ter sido tagueada antes
do bump).
**Melhorias pós-v0.2.0 (2026-06-18):** **QoL de tempo** (`--day N`, modo história `--story`),
**transição opt-in** (`--transition dissolve`, o dissolve LFSR dormente do original),
**Web/WASM** (crate `wilson-web`: engine no navegador — traga seus `RESOURCE.*` soltos **ou um
`.zip`** run/instalador, ou embutido; **som** ligado por padrão 🔊+volume (Web Audio), **tela
cheia ⛶** com fundo preto + **Wake Lock**, **opções na URL** espelhando o desktop
[`?fullscreen/scale/filter/speed/day/dissolve/story/daynight/intro/mute/volume/seed`] com
**`scale=fit`+`filter=linear` por padrão** como o desktop, e **salvar os dados no navegador**
(IndexedDB, opt-in, com "Forget"); API `Options` + `Wilson.create/from_zip/embedded`; build
**separado** via `crates/wilson-web/build-web.sh` — o `build-embedded.sh` é só desktop), e
**pathfinding byte-fiel** (`calcpath` portado dos route streams ponderados do original via
`docs/reverse-engineering/extract_calcpath.py` → `calcpath_data.rs`; corrigiu de quebra um bug
de RNG com seeds pequenas).
**Fix (2026-06-18):** o **intro passava rápido demais no desktop** — o loop winit avançava o
engine em todo `RedrawRequested` (inclusive os espontâneos do SO), atropelando o `WaitUntil`;
agora `pace::FramePacer` trava o avanço por deadline (teste de regressão fail-first). RE
confirmou que o original **não tem timer de intro** (hard-cut, fica até a 1ª cena carregar).
**Intro polish (2026-06-18):** duração do intro agora **3 s por padrão e configurável**
(`DEFAULT_INTRO_TICKS=187`, `enable_intro(archive, ticks)`, `intro_ticks_from_secs`; desktop
`--intro-secs 1–30`/config `intro_secs`, web `?intro_secs`). **Dissolve no intro = opt-in**: com
`--transition dissolve`/`?dissolve` ligado, dissolve do `INTRO.SCR`→1ª cena (`Show::intro_boundary`).
**Byte scan do binário inteiro provou** que o dissolve está morto (gate `[0x1ebf]`: 10 leituras, 0
escritas) ⇒ o original **nunca** dissolve, nem no intro — padrão segue hard-cut fiel (KB10 §10.2).
**Fix Windows .scr (2026-06-18):** testando no Windows 11 — (1) faltava
`#![windows_subsystem = "windows"]` (o `.scr` abria janela de **console** preta); (2) **Configurar**
(`/c`) só fazia `println!` ⇒ agora `configure()` abre o `config.txt` no editor; (3) o **preview**
(`/p`) tocava som ⇒ `audio_muted` silencia o preview. Validado por cross-compile `windows-gnu`
(confirmação final no Windows do usuário).
**Fix preview (Windows, 2026-06-18):** o monitorzinho das Configurações **renderiza** (não fica
preto), mas a janela filha estava fixa em 152×112 ⇒ faixas pretas à direita/embaixo no painel maior
do Win11. Agora `apply_preview` consulta `GetClientRect(hwnd)` (FFI `user32`, sem dep nova) e
**preenche o painel**. Cross-compile `windows-gnu` OK; runtime confirmado pelo usuário no Windows.
**v0.3.1 (2026-06-19):** segunda leva pós-0.3.0 — **preview do Windows preenche o painel**
(`GetClientRect`); **UX web zero-fricção** (auto-save+auto-start; **📁 removido ⇒ só 🗑 Forget**;
picker sempre visível); **F11 do navegador = botão ⛶** (detecção via `(display-mode: fullscreen)`,
teste de regressão no `e2e/run.mjs`); **GitHub Pages ao vivo**
(<https://antigerme.github.io/wilson-reborn/>) + link nos docs; **CI 1×/PR**; e **doc de "abrir
direto em tela cheia"** (`--kiosk`/`--start-fullscreen`, validado em Chrome *headed* sob Xvfb).
**Próximo:** taguear `v0.3.1` (passo do usuário) e acompanhar o `release.yml` até os artefatos.
