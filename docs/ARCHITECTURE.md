<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Arquitetura do Wilson Reborn (mapa para devs e IAs)

Este documento dá o **modelo mental** do projeto: como os dados viram pixels na tela,
o papel de cada crate/módulo, a correspondência com o `jc_reborn` (a referência) e
**como validar** sem ficar caçando erro a olho. Leia junto com o `CLAUDE.md`.

## O pipeline, do disco à tela

```
RESOURCE.MAP + RESOURCE.001  (dados originais do jogo, 1992 — copyright, não versionados)
        │
        ▼
  wilson-dgds        decodifica o formato DGDS/SCRANTIC:
                     mapa/arquivo, RLE/LZW, chunks, paleta, BMP/SCR/TTM/ADS,
                     e o bytecode TTM/ADS → instruções tipadas. Zero deps. Pura.
        │  (Archive, Palette, Ttm, Ads, ResourceMap, find_ci, …)
        ▼
  wilson-engine      executa os scripts contra uma Surface indexada (headless):
                     • ttm_exec  — interpreta um frame de um thread TTM (opcodes → desenho)
                     • ads_vm    — escalona até 10 threads TTM concorrentes (o "scene player")
                     • story     — o diretor: 63 cenas, ciclo de 11 dias, maré/noite/feriado,
                                   deriva da ilha + filtro VARPOS_OK/LEFT_ISLAND
                     • walk/path — Johnny anda entre "spots" (pathfinding + animação)
                     • island    — desenha a ilha/água/nuvens/raft no fundo
                     • show      — junta tudo: planeja a run, anda, toca a cena, compõe frames
        │  (Show::next_frame → Frame { surface, delay_ticks, sounds })
        ▼
  wilson (app)       winit + softbuffer: abre a janela, e a cada timer desenha o Frame
                     (Surface → RGBA via paleta → escala 4:3 com letterbox). Carrega os
                     dados originais (--data/auto), toca soundN.wav, persiste dia/stats.
```

`wilson-saver` é a mesma engine exposta via FFI para o screensaver nativo do macOS.

## O laço de produção de frames (o coração)

`Show::next_frame` espelha o `storyPlay` do jc_reborn:
1. **Planeja uma run** (`Director::plan_run`): escolhe a cena final, a cadeia de cenas
   ambiente (6–20), e o estado da ilha (maré/noite/feriado/raft + deriva `x_pos/y_pos`).
2. Para cada cena: **anda** Johnny até o spot (`Walker`), depois **toca** a cena ADS
   (`AdsVm`) sobre o fundo da ilha.
3. `AdsVm::next_frame` faz **uma iteração do escalonador**: avança os threads cujo timer
   zerou, compõe `fundo → zonas salvas → camadas dos threads`, e devolve `delay_ticks =
   mini` (o menor delay pendente entre os threads ativos).
4. Sem mais cenas → planeja a próxima run.

## Tempo (a parte que já nos mordeu)

- Unidade: **1 tick = 20 ms**. O frame carrega `delay_ticks`; o app espera
  `frame_delay_ms(ticks) = ticks * 20 * 100 / speed` ms (`config.rs`).
- Isso é **idêntico** ao jc_reborn: `eventsWaitTick(delay)` faz `delay *= 20`
  (`events.c`), com `grUpdateDelay = mini` (`ads.c`).
- ⚠️ O ritmo só fica certo se o app **respeitar** `delay_ticks`. O loop do winit deve
  redesenhar **apenas** quando o timer estoura (`StartCause::ResumeTimeReached`), nunca a
  cada `AboutToWait` (isso atropela o `WaitUntil` e roda acelerado). Ver `main.rs`.

## Deriva da ilha (a parte do "balão fora da tela")

- A ilha pode derivar: `island_from_scene` sorteia `x_pos/y_pos` (faixas de `story.c`).
- Quando a ilha está deslocada, o diretor exige cenas com a flag **`VARPOS_OK`**
  (`wanted |= VARPOS_OK`, = `story.c:230`): só cenas que ficam OK deslocadas entram.
- O primeiro plano (Johnny + props da cena) é desenhado em `ttmDx/ttmDy = x_pos/y_pos`
  (`+272` se a cena tem `LEFT_ISLAND`) — então Johnny acompanha a ilha. Ver `ads_offset`.

## Como validar (em vez de caçar erro a olho)

**Rede automática (no CI, sem dados originais):**
- `cargo test -p wilson-engine` — entre eles `engine_run_stays_live_and_paced`: roda
  milhares de frames e exige que nunca paniqueie, sempre emita 640×480, **continue
  animando** (não congela) e mantenha um **ritmo humano**.

**Validação profunda (local, com os dados originais):**
```bash
WILSON_DATA_DIR=<dir-com-os-dados> cargo test -p wilson-engine real_data_long_run_invariants -- --nocapture
```
Simula ~20 min de playback avançando o calendário e exige: frames 640×480, **100%
opacos** (sem TRANSPARENT vazado — a classe do "água magenta"), animação viva, ritmo
humano e o **dia avançando**.

**Revisão visual (humano ou IA olha um mosaico amostrado):**
```bash
# renderiza ~1h de run e salva 1 frame a cada ~30s (cabe em disco)
cargo run -p wilson-engine --example render_run -- <dir-dados> /tmp/out 27000 225 1
# vira um mosaico (ou um mp4 curto) com ffmpeg:
ffmpeg -pattern_type glob -i '/tmp/out/*.ppm' -vf 'scale=240:180,tile=8x8' /tmp/montage.png
```
Os quadros são exatamente o que o app mostra. Olhar o mosaico pega erros visuais grossos
(ilha fora da tela, cor errada, congelamento) que invariantes não capturam.

> Limite honesto: sem um vídeo de referência do original (não-determinístico), não dá
> para comparar pixel a pixel. A combinação **invariantes + revisão de mosaico + teste
> real ocasional** é o que mantém a coisa "100% funcional" sem inspeção manual constante.
