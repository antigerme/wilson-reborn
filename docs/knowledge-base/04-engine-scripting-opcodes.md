# 04 — Motor de Scripting: Opcodes TTM e ADS

> O conteúdo do Johnny Castaway é **dirigido por bytecode**. Há dois níveis:
> - **TTM** (*"Tableau"/Text-Tableau Movie*) = bytecode de **animação por cena**
>   (desenha sprites/primitivas, define delays, toca som). Fica no chunk `TT3:` de um
>   `.TTM`.
> - **ADS** (*Animation/Director Script*) = bytecode de **sequenciamento de cenas** —
>   decide *quais* TTMs tocar, com condicionais e aleatoriedade. Fica no chunk `SCR:`
>   de um `.ADS`.
>
> Reproduzir fielmente estes dois interpretadores = reproduzir ~95% do comportamento.
> Esta é a "joia da coroa" da engenharia reversa.
>
> **Fonte primária:** `jc_reborn` (`ttm.c`, `ads.c`, `dump.c`) — o mais completo e
> específico do JC. **Referências cruzadas:** JCOS (`Instruction.cs` tem os operandos
> exatos via disassembler), castaway/dgds-viewer (`process.*`), ScummVM (`dgds.cpp`).
> Tabelas completas: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md) §3–§4 e
> [`raw/jcos-csharp-notes.md`](raw/jcos-csharp-notes.md).

---

## 1. Codificação das instruções

### TTM (`ttm.c:141`)
```
u16 opcode (LE)
numArgs = opcode & 0x000F          ; nº de operandos int16
op      = opcode & 0xFFF0          ; o opcode em si (nibble baixo zerado)
SE numArgs == 0x0F:                ; caso especial: 1 operando STRING
    lê pares de bytes (UTF-16-ish) até o par 00 00 (ASCIIZ, padding par)
SENÃO:
    lê numArgs × int16 (LE)
```
**Regra de ouro:** o nibble baixo do opcode é a **contagem de argumentos**; `0xF`
significa "um argumento string" (usado por `LOAD_*`).

### ADS (`ads.c`)
```
u16 opcode (LE)
SE (opcode & 0xFF00) == 0:         ; valor pequeno
    é um TAG/ID (push de id de cena/sequência)
SENÃO:
    é um opcode de 16 bits; nº de operandos é FIXO por opcode (não vem do nibble)
```

---

## 2. Tabela de opcodes TTM

Conforme implementado no `jc_reborn` (coluna "Efeito"). `[dump-only]` = reconhecido pelo
disassembler mas **sem handler de runtime** no jc_reborn (provavelmente irrelevante para
o JC, mas existe nos dados). Hex já inclui o nibble de contagem.

| Opcode | Mnemônico | Args | Efeito |
|---|---|---:|---|
| `0x001F` | SAVE_BACKGROUND | str | [dump-only] |
| `0x0080` | DRAW_BACKGROUND | 0 | stub (libera slots de imagem) |
| `0x0110` | PURGE | 0 | controle de fim/loop: se `sceneTimer` ativo → salta p/ tag anterior; senão marca thread como finalizada |
| `0x0FF0` | **UPDATE** | 0 | **yield**: encerra o passo atual, apresenta o frame |
| `0x1021` | **SET_DELAY** | 1 | `timer = delay = max(arg, 4)` — atraso do frame em **ticks** |
| `0x1051` | SET_BMP_SLOT | 1 | seleciona slot de spritesheet para os próximos desenhos |
| `0x1061` | SET_PALETTE_SLOT | 1 | reconhecido, no-op (paleta global única) |
| `0x1101` | LOCAL_TAG | 1 | marcador de tag (bookmark no load) |
| `0x1111` | TAG | 1 | marcador de cena/label (bookmark no load) |
| `0x1121` | TTM_UNKNOWN_1 | 1 | define região usada antes de SAVE_IMAGE1/CLEAR_SCREEN; no-op |
| `0x1201` | **GOTO_TAG** | 1 | `nextGotoOffset = ttmFindTag(arg)` (salto diferido) → loops |
| `0x2002` | SET_COLORS | 2 | `fgColor=arg0; bgColor=arg1` |
| `0x2012` | SET_FRAME1 | 2 | sempre (0,0) perto de LOAD_IMAGE; no-op |
| `0x2022` | TIMER | 2 | `delay = timer = (arg0+arg1)/2` (aproximação) |
| `0x4004` | SET_CLIP_ZONE | 4 | `grSetClipZone(x1,y1,x2,y2)` |
| `0x4110` | FADE_OUT | 0 | [dump-only] |
| `0x4120` | FADE_IN | 0 | [dump-only] |
| `0x4204` | COPY_ZONE_TO_BG | 4 | blita região na camada persistente de "zonas salvas" |
| `0x4214` | SAVE_IMAGE1 | 4 | define zona de redesenho (efetivamente no-op) |
| `0xA002` | DRAW_PIXEL | 2 | `grDrawPixel(x,y,fgColor)` |
| `0xA054` | SAVE_ZONE | 4 | `grSaveZone` (só GJGULIVR.TTM) |
| `0xA064` | RESTORE_ZONE | 4 | `grRestoreZone` |
| `0xA0A4` | DRAW_LINE | 4 | linha Bresenham `(x1,y1,x2,y2,fgColor)` |
| `0xA104` | DRAW_RECT | 4 | retângulo preenchido `(x,y,w,h,fgColor)` |
| `0xA404` | DRAW_CIRCLE | 4 | círculo `(x,y,w,h,fg,bg)` |
| `0xA504` | **DRAW_SPRITE** | 4 | `grDrawSprite(x,y,spriteNo,imageNo)` |
| `0xA510` | DRAW_SPRITE1 | 0 | [dump-only] |
| `0xA524` | **DRAW_SPRITE_FLIP** | 4 | sprite espelhado horizontalmente |
| `0xA530` | DRAW_SPRITE3 | 0 | [dump-only] |
| `0xA601` | CLEAR_SCREEN | 1 | limpa a camada desta thread |
| `0xB606` | DRAW_SCREEN | 6 | reconhecido, no-op |
| `0xC020` | LOAD_SAMPLE | 0 | [dump-only] |
| `0xC030` | SELECT_SAMPLE | 0 | [dump-only] |
| `0xC040` | DESELECT_SAMPLE | 0 | [dump-only] |
| `0xC051` | **PLAY_SAMPLE** | 1 | `soundPlay(arg0)` |
| `0xC060` | STOP_SAMPLE | 0 | [dump-only] |
| `0xF01F` | **LOAD_SCREEN** | str | carrega `.SCR` como fundo |
| `0xF02F` | **LOAD_IMAGE** | str | carrega `.BMP` no slot selecionado |
| `0xF05F` | LOAD_PALETTE | str | reconhecido, no-op (paleta global já carregada) |

### Como uma animação roda
Uma "cena" é um ponto de entrada (tag) dentro do TTM. Cada passo de `ttmPlay()`
tipicamente: `CLEAR_SCREEN` na camada transparente da thread → desenha sprites/primitivas
de **um frame** → `UPDATE` (yield). O escalonador espera `delay` ticks e reentra;
`GOTO_TAG`/`PURGE` criam loops. A composição final é camada sobre camada
(fundo → zonas salvas → cada thread → camada de feriado).

---

## 3. Tabela de opcodes ADS

Do `jc_reborn` (`ads.c` runtime + `dump.c` nomes). Args em palavras de 16 bits.

| Opcode | Mnemônico | Args | Significado |
|---|---|---:|---|
| `0x1070` | IF_LASTPLAYED_LOCAL | 2 | "se última tocada (slot,tag)" local; enfileira chunk local (só ACTIVITY.ADS tag 7) |
| `0x1330` | IF_UNKNOWN_1 | 2 | guarda, sinônimo de IF_NOT_RUNNING; ignorado no runtime |
| `0x1350` | **IF_LASTPLAYED** | 2 | **gatilho reativo**: chunk roda quando a cena (slot,tag) termina |
| `0x1360` | IF_NOT_RUNNING | 2 | se (slot,tag) rodando → pula o bloco |
| `0x1370` | IF_IS_RUNNING | 2 | pula o bloco a menos que (slot,tag) esteja rodando |
| `0x1420` | AND | 0 | AND booleano de condições |
| `0x1430` | OR | 0 | OR booleano (`inOrBlock`) |
| `0x1510` | PLAY_SCENE | 0 | "fecha-chaves" de um bloco condicional |
| `0x1520` | ADD_SCENE_LOCAL | 5 | adiciona cena enfileirada por gatilho local |
| `0x2005` | **ADD_SCENE** | 4 | **cria thread TTM** (slot,tag,arg3,?). Em bloco RANDOM → candidato com peso |
| `0x2010` | STOP_SCENE | 3 | para cena por (slot,tag) |
| `0x2014` | UNKNOWN_5 | 0 | reconhecido pelo dumper |
| `0x3010` | **RANDOM_START** | 0 | início de bloco de seleção aleatória ponderada |
| `0x3020` | NOP | 1 | candidato "não fazer nada" (o peso é o arg) |
| `0x30FF` | **RANDOM_END** | 0 | escolhe e executa **um** candidato por peso |
| `0x4000` | UNKNOWN_6 | 3 | só BUILDING.ADS tag 7; ignorado |
| `0xF010` | FADE_OUT | 0 | reconhecido, no-op no runtime |
| `0xF200` | GOSUB_TAG | 1 | chama o chunk de outra tag inline (ex.: STAND.ADS→tag 14) |
| `0xFFFF` | **END** | 0 | fim da sequência → pede parada |
| `0xFFF0` | END_IF | 0 | fecha um bloco IF |
| (outro) | `:TAG n` | 0 | qualquer outro valor = id de tag/label |

### Semântica de `ADD_SCENE` (arg3) — **load-bearing**
- `arg3 < 0` → toca **por `-arg3` ticks** (`sceneTimer`);
- `arg3 > 0` → toca **`arg3` vezes** (`sceneIterations`);
- `arg3 == 0` → toca **uma vez** até o fim natural.

### Modelo de execução ADS (encadeamento reativo)
1. `adsLoad()` pré-escaneia o script: marca tags e os **chunks de gatilho**
   (`IF_LASTPLAYED`/`IF_NOT_RUNNING`).
2. Toca o chunk inicial → `ADD_SCENE` cria threads TTM.
3. Quando uma thread TTM **termina**, `adsPlayTriggeredChunks()` dispara qualquer chunk
   cujo `IF_LASTPLAYED (slot,tag)` casa — **é assim que uma animação encadeia na
   próxima**.
4. `RANDOM_START … RANDOM_END` escolhe uma operação por **peso** (soma os pesos,
   `rand()%total`, percorre a distribuição cumulativa) — é a base do "Johnny escolhe
   aleatoriamente o que fazer".

---

## 4. Divergências entre implementações (atenção ao portar)

As reimplementações nem sempre concordam (foram engenharia reversa independente). Onde
divergem, **prefira o `jc_reborn`** (específico do JC) e valide rodando contra os dados:

| Tema | jc_reborn (JC) | ScummVM (RotD/HoC) | JCOS / castaway |
|---|---|---|---|
| Unidade de DELAY | tick = **20 ms** (`events.c:108`); `SET_DELAY` em ticks | `0x1020`: `delay += arg*10` ms | JCOS: ×20 ms |
| `0xA100` | **DRAW_RECT** (preenchido) | "SET (bmp) WINDOW" | castaway: DRAW_RECT; JCOS: SET_WINDOW0 |
| `0x4000` (TTM) | SET_CLIP_ZONE | SET WINDOW | castaway: SET_CLIP_REGION |
| Nibble = contagem | sim | sim | sim (JCOS mascara `& 0xfff0`) |
| LZW: tamanho tabela | 4096 | aloca 16384 (cap real 12 bits=4096) | JCOS: 4096 |
| Opcodes ADS implementados | quase todos | só `0x2005` (resto stub) | maioria |
| Colisões de opcode TTM×ADS | resolvidas por contexto | — | castaway: 1ª correspondência; viewer: última-chave |

> **Colisões reais:** `0x2010`, `0x4000`, `0xF010` significam coisas diferentes em TTM
> vs ADS. O interpretador deve despachar conforme o **tipo de script** corrente.

> **JCOS como dicionário:** o `Instruction.cs` do JCOS tem um `ToString()` que soletra os
> operandos de ~60 opcodes — é a melhor referência para **nomes e tuplas de operandos**.
> Sprites: JCOS confirma `0xA500/0xA520 = DRAW_SPRITE/DRAW_SPRITE_FLIP` (o `2` = espelhado)
> e que `DRAW_SPRITE1/3` (`0xA510/0xA530`) lançam "não implementado".

---

## 5. Implicações para o Wilson Reborn

1. **Transcreva os dois VMs** (TTM e ADS) com as tabelas acima. São pequenos e bem
   compreendidos.
2. **Preserve as convenções load-bearing:** sinal do `arg3` de ADD_SCENE; tri-estado
   `isRunning` (0 livre / 1 rodando / 2 finalizada-pendente / 3 fundo-estático); nibble =
   contagem; `0xF`=string.
3. **Despache por contexto** (TTM vs ADS) para resolver colisões de opcode.
4. **Padronize o tick em 20 ms** e o **escalonador cooperativo de timestep variável**
   (ver [05](05-arquitetura-do-engine.md) §loop).
5. **Modernização possível:** os opcodes `[dump-only]` (FADE_IN/OUT, C0xx de som,
   SAVE_BACKGROUND) podem ser **implementados de verdade** no Wilson Reborn para fidelidade
   extra, já que o hardware moderno permite (o jc_reborn os ignorou por simplicidade).
