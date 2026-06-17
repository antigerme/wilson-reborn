# 05 — Arquitetura do Engine (loop, diretor, walk, ilha, render, som)

> Como as peças se encaixam em tempo de execução. Baseado no `jc_reborn` (a
> implementação mais completa e fiel). file:line apontam para `repos/jc_reborn/`.
> Notas completas: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md).

```
                    storyPlay()  [story.c]  ← o DIRETOR (loop infinito)
                        │  escolhe cenas (dia/maré/feriado), caminha entre spots
                        ▼
                    adsPlay()    [ads.c]    ← o ESCALONADOR (interpreta 1 cena ADS)
                        │  gerencia até 10 threads TTM + thread de fundo + feriado
                        ▼
                    ttmPlay()    [ttm.c]    ← a VM de animação (1 frame de 1 thread)
                        │  desenha na camada (SDL surface) da thread
                        ▼
                 grUpdateDisplay() [graphics.c] ← compõe camadas + espera tick + apresenta
                        │
                 eventsWaitTick()  [events.c]   ← o batimento: 1 tick = 20 ms
```

---

## 1. Ponto de entrada e modos — `jc_reborn.c`

`main()` (`jc_reborn.c:152`): `parseArgs()`, sempre
`parseResourceFiles("RESOURCE.MAP")`, depois despacha:
- **default** → `graphicsInit(); soundInit(); storyPlay();` (o screensaver real, loop
  infinito);
- `dump` → extrai todos os recursos para `./dump/`;
- `bench` → benchmark de fps;
- `ttm <nome>` → toca um TTM direto;
- `ads <nome> <tag>` → toca uma cena ADS.

Hot-keys (quando habilitadas): `Esc`=sair, `Alt+Return`=fullscreen, `Espaço`=pausar,
`Return`=avançar 1 frame (pausado), `M`=velocidade máx/normal. **Sem** hot-keys (modo
screensaver): **qualquer tecla encerra** (`exit(255)`).

---

## 2. Modelo de tempo — `events.c` (o batimento)

Tudo é medido em **ticks**; **1 tick = 20 ms** (`eventsWaitTick`: `delay *= 20`,
`events.c:108`). Taxa nominal = **50 ticks/s**. Um valor `delay` N num TTM = N×20 ms.

`eventsWaitTick(delay)` faz busy-wait (granularidade `SDL_Delay(5)`) até
`SDL_GetTicks() - lastTicks >= delay*20`, enquanto sonda eventos SDL. Respeita
`paused`/`oneFrame` (step) e `maxSpeed` (tecla M ignora a espera).

---

## 3. O escalonador — `adsPlay()` (`ads.c:658`)

É o loop principal **enquanto uma cena toca**. Modelo **cooperativo, dirigido por
eventos, de timestep variável** — dorme exatamente até a próxima thread precisar de
serviço (não é frame fixo):

`while (numThreads)`:
1. Se a thread de **fundo** (ondas) está na hora (`timer==0`) → `islandAnimate()`.
2. Para cada uma das `MAX_TTM_THREADS` (**10**) threads TTM com `timer==0` → `ttmPlay()`.
3. `grUpdateDisplay(...)` (compõe + espera).
4. `mini` = menor `timer` pendente entre as threads (cap 300); decrementa todos os
   `timer` por `mini`; `grUpdateDelay = mini`.
5. Pós-processamento por thread: aplica `nextGotoOffset`; decrementa `sceneTimer`
   (ADD_SCENE negativo); ao terminar (`isRunning==2`) re-arma `sceneIterations` vezes
   (ADD_SCENE positivo) ou para e dispara `IF_LASTPLAYED`/`IF_NOT_RUNNING`
   (`adsPlayTriggeredChunks`).

**`isRunning` (tri-estado+):** `0`=slot livre; `1`=rodando; `2`=terminou neste passo
(limpeza pendente); `3`=fundo/feriado (desenhado, não "steppado").

Constantes: `MAX_TTM_SLOTS=10`, `MAX_TTM_THREADS=10`, `MAX_ADS_CHUNKS=100`,
`MAX_RANDOM_OPS=10`.

---

## 4. O diretor de história — `story.c`

`storyPlay()` (`story.c:194`): `adsInit(); adsPlayIntro();` e então, para sempre:
1. `storyUpdateCurrentDay()` + `storyCalculateIslandFromDateAndTime()`.
2. Escolhe uma cena **FINAL** (o gag clímax da rodada) via `storyPickScene(FINAL,0)`.
3. Se for cena `ISLAND` → calcula parâmetros da ilha e `adsInitIsland()`; senão
   `adsNoIsland()`.
4. A menos que a final seja também `FIRST`, toca uma cadeia de **`6 + rand()%14`** cenas
   ambiente que levam até ela. Entre cenas consecutivas, o Johnny **caminha** do
   spot/heading final da anterior ao inicial da próxima (`adsPlayWalk`). Cenas de
   enredo (`dayNo≠0`) disparam `soundPlay(0)`.
5. Caminha até a final, toca, `grFadeOut()`, libera a ilha.

### Seleção de cena — `storyPickScene(wanted, unwanted)` (`story.c:42`)
Coleta toda cena cujas flags contêm todas as `wanted`, nenhuma das `unwanted`, e cujo
`dayNo` é 0 **ou** igual ao `storyCurrentDay`; retorna uma **uniformemente aleatória**.

### Avanço do dia — `storyUpdateCurrentDay()` (`story.c:65`)
**Dirigido pelo relógio real e persistido** em `~/.jc_reborn` (`config.c`: `currentDay=`,
`date=`). Se o dia do calendário (`getDayOfYear()`/`tm_yday`) difere do gravado →
`currentDay += 1`; clamp/wrap em **1..11**. Ou seja: a história avança **um beat por dia
real** e repete a cada 11 dias.

> Tabela completa de 63 cenas (spots/headings/dia/flags) e o mapa dia→cena estão na
> [bíblia de conteúdo](02-biblia-de-conteudo.md) §2/§13 e em
> [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md) §7.

### Estado da ilha derivado da cena — `storyCalculateIslandFromScene()` (`story.c:123`)
- **Maré baixa:** se `LOWTIDE_OK` e `rand()%2`.
- **Posição da ilha:** se `VARPOS_OK`, sorteia 1 de 3 faixas de offset; senão fixa
  (`LEFT_ISLAND` → `xPos=-272`, senão 0).
- **Progresso da jangada:** 0 se `NORAFT`; senão por dia (0–2→1; 3–5→dia−1; ≥6→5).
- `HOLIDAY_NOK` (só VISITOR.ADS#3, o cargueiro) força `holiday=0`.

### Dia/noite e feriados — `storyCalculateIslandFromDateAndTime()` (`story.c:94`)
- **Noite:** `hour = getHour()%8; night = (hour==0 || hour==7)` → carrega `NIGHT.SCR`.
- **Feriados** (comparação de string `MMDD`): Halloween 29–31/10 (1), S. Patrício
  15–17/3 (2), Natal 23–25/12 (3), Ano Novo 29/12–01/01 (4). Detalhes e props em
  [bíblia §9](02-biblia-de-conteudo.md#9-datas-comemorativas--feriados-annivers--lógica-de-storyc).

---

## 5. Caminhada e pathfinding

Johnny se move entre **6 spots nomeados A–F** (os mesmos nós da tabela de história).
Movimento = escolher rota no grafo de spots, depois tocar frames de animação pré-gerados.

### Pathfinding — `calcpath.c` + `calcpath_data.h`
`NUM_OF_NODES=6`. `walkMatrix[7][6][6]` é uma **adjacência de segunda ordem**:
`walkMatrix[prev][cur][next]` = 1 se pode ir cur→next tendo vindo de prev (restrições de
curva que evitam ré abrupta). O índice `[6]` = "de qualquer spot" (primeiro salto).
`calcPath(from,to)` faz **DFS enumerando todos os caminhos simples** (até
`MAX_NUM_PATHS=50`, `MAX_PATH_LEN=7`) e retorna **um aleatório**. (O autor admite que é
um ajuste plausível, não o algoritmo original.)

### Animação — `walk.c` + `walk_data.h`
`walkData[][4]` = tabela de frames `{flip, x, y, spriteNo}`, segmentada por rota
(A→E, A→F, …) com sentinelas `{0,0,0,0}`. **Extraída do executável `SCRANTIC.SCR`**
(offset `0x188ea`), não dos recursos. Tabelas de índice: `walkDataBookmarks[6][6]`
(início de cada rota), `walkDataBookmarksTurns[6]` (frames de virada por spot),
`walkDataStart/EndHeadings[6][6]`.

`walkAnimate()` é uma máquina de estados (virar → andar → chegar) que retorna o **delay**
até o próximo frame (0 ao chegar). Detalhe: ao andar entre **D↔E**, o Johnny passa
**atrás da palmeira** — o engine redesenha tronco (sprite 13 @442,148) e folhas (12
@365,122) por cima dele. Sprites vêm de `JOHNWALK.BMP`.

---

## 6. Fundo da ilha — `island.c`

`TIslandState islandState` (`island.h:24`): `{lowTide, night, raft, holiday, xPos, yPos}`
— o estado global do cenário atual.

`islandInit()` (`island.c:35`): escolhe o fundo (`NIGHT.SCR` ou `OCEAN0{0,1,2}.SCR`,
`rand%3`) e **pinta a cena estática direto na surface de fundo** (para os TTM/walk
comporem por cima): jangada (estágios 0–5 de `MRAFT.BMP`), nuvens (`BACKGRND.BMP`
sprites 15–17, 0–5 delas, espelhadas conforme o "vento"), ilha (sprite 0 @288,279),
tronco (13), folhas (12), sombra (14), e, na maré baixa, faixa de areia (1) + rocha (2).

`islandAnimate()` (`island.c:150`): anima as ondas da praia (3 fases). Maré alta → 3
posições (sprites 3/6/9); maré baixa → 4 (30/33/36/39). **É a única animação contínua
de fundo**, dirigida pela thread de fundo do escalonador (`delay=8`).

**Legenda de sprites `BACKGRND.BMP`:** 0 ilha, 1 areia maré-baixa, 2 rocha, 3/6/9 ondas
maré-alta (3 fases), 12 folhas, 13 tronco, 14 sombra, 15/16/17 nuvens, 30/33/36 ondas
maré-baixa, 39 ondas na rocha. **`MRAFT.BMP`** imagens 0–4 = estágios 1–5 da jangada.

Props de feriado — `islandInitHoliday()` (`island.c:192`): carrega `HOLIDAY.BMP` e
desenha (numa camada `isRunning=3`): Halloween→sprite 0 @(410,298); S. Patrício→1
@(333,286); Natal→2 @(404,267); Ano Novo→3 @(361,155).

---

## 7. Gráficos — `graphics.c`

- **Resolução lógica fixa 640×480**, janela SDL 32-bpp. Sem escalonamento (origem
  `{0,0}`). *(Este é o principal ponto a modernizar — ver [07](07-plano-do-port-moderno.md).)*
- **Paleta:** as 16 primeiras cores do `.PAL` (VGA 6-bit) convertidas para RGBA por
  `<<2`. Armazenamento **BGR** no jc_reborn.
- **Camadas / double-buffer:** cada thread TTM renderiza na sua própria surface
  off-screen 640×480, preenchida com **magenta `0xA8,0x00,0xA8`** como cor-chave de
  transparência. `grUpdateDisplay()` compõe na ordem: **fundo → zonas salvas → cada
  thread rodando → camada de feriado**; espera o tick; `SDL_UpdateWindowSurface`.
- **Primitivas:** pixel, linha (Bresenham), retângulo preenchido, círculo, clip-zone.
  Todas respeitam o offset `grDx/grDy` (ilha-relativo) e escrevem na camada dada.
- **Sprites:** `grLoadBmp()` decodifica cada sub-imagem para surface 32-bpp com cor-chave
  magenta; `grDrawSprite` blita em `(x+grDx, y+grDy)`; `grDrawSpriteFlip` espelha
  coluna a coluna.
- **Fade-out:** `grFadeOut()` alterna 5 estilos de transição (círculo expandindo, retângulo
  expandindo, E→D, D→E, do meio) a cada chamada.

> **Aproximações conhecidas:** `grSaveImage1`/`grSaveZone` são quase-stubs;
> `grUpdateDisplay` re-blita tudo a cada frame (sem dirty-rects) — irrelevante numa GPU
> moderna.

---

## 8. Som — `sound.c`

`NUM_OF_SOUNDS=25`. `soundInit()` abre o áudio SDL e carrega `sound%d.wav` (i=0..24;
faltantes toleráveis). Mixer de software de **1 canal** (`soundCallback`): um som "atual"
por vez. `soundPlay(nb)` define o ponteiro sob `SDL_LockAudio`. **Sound 0** = cue genérico
de transição de cena de enredo; os demais vêm de `PLAY_SAMPLE` (`0xC051`) no TTM.

---

## 9. Persistência — `config.c`

Arquivo texto `~/.jc_reborn` (ou CWD se não houver `$HOME`): `currentDay=N` e `date=N`.
Usado só pelo mecanismo de dia da história (§4). **No Wilson Reborn**, equivale a um
arquivo de config multiplataforma (ver [07](07-plano-do-port-moderno.md)).

---

## 10. O que portar verbatim vs. o que é data-driven

**Portar verbatim (não vem do `RESOURCE.001`):**
- `story_data.h` — 63 cenas (ads/tag/spots/headings/dia/flags);
- `walk_data.h` — frames de caminhada + bookmarks (do `SCRANTIC.SCR`);
- `calcpath_data.h` — matriz de adjacência de 2ª ordem;
- as **tabelas de opcodes** ([04](04-engine-scripting-opcodes.md)) e a **lógica de
  feriado/dia/noite/maré/posição** (§4).

**Data-driven (vem dos arquivos):** todas as animações, sprites, telas, paletas, sons,
e a estrutura das cenas (via bytecode TTM/ADS).

**Camadas limpas (manter no port):** I/O (`resource`/`uncompress`) · VM
(`ttm`/`ads`) · backend (`graphics`/`sound`) · lógica de jogo
(`story`/`events`/`island`/`walk`). Trocar SDL por qualquer backend é direto.
