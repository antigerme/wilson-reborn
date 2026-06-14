# 03 — Dados Originais e Formatos de Arquivo (DGDS / SCRANTIC)

> Especificação byte a byte do formato de dados do Johnny Castaway, para o Wilson
> Reborn **carregar os arquivos originais** (`RESOURCE.MAP` + `RESOURCE.001`).
>
> Fontes: `repos/jc_reborn` (`resource.c`, `uncompress.c`) e `repos/castaway`
> (`docs/resindex.md`, `src/dgds/*`) — autoritativos para **Johnny Castaway**; e o
> engine DGDS do ScummVM (`repos/dgds`) — autoritativo para a **família DGDS**
> (Rise of the Dragon, Heart of China). Onde JC difere da família, está marcado.
> Notas brutas completas: [`raw/jc_reborn-notes.md`](raw/jc_reborn-notes.md),
> [`raw/dgds-scummvm-notes.md`](raw/dgds-scummvm-notes.md),
> [`raw/jcos-csharp-notes.md`](raw/jcos-csharp-notes.md).

---

## 1. Visão geral em camadas

```
RESOURCE.MAP  (índice)  ─┐
                         ├─► lista de entradas { tamanho_descomprimido, offset }
RESOURCE.001  (arquivo) ─┘     │
   └─ cada entrada = [nome 13b][tamanho u32][ dados em CHUNKS ]
                                                   │
            cada chunk = ["XXX:"][size u32] (+ se "packed": [método][unpackSize u32][corpo])
                                                   │
                  corpo descomprimido (None / RLE / LZW)
                                                   │
       interpretado conforme o TIPO do recurso (.ADS .TTM .SCR .BMP .PAL ...)
```

São necessários **apenas dois codecs** (RLE e LZW) e o **container de chunks com tag
ASCII** para ler tudo de que o Johnny precisa.

---

## 2. Arquivos originais necessários

| Arquivo | Tamanho (bytes) | MD5 | Papel |
|---|---:|---|---|
| `RESOURCE.MAP` | 1.461 | `8bb6c99e9129806b5089a39d24228a36` | Índice de recursos |
| `RESOURCE.001` | 1.175.645 | `374e6d05c5e0acd88fb5af748948c899` | Arquivo com todos os recursos |
| `SCRANTIC.SCR` | — | — | **Executável** do screensaver (contém o engine + tabela de caminhada do Johnny embutida — ver §8). Não confundir com recursos `.SCR` internos. |
| `sound0.wav` … `sound24.wav` | vários | (ver `repos/jc_reborn/README.md`) | 24 efeitos sonoros (extraídos por JCOS; ver §7) |

> **Importante:** o nome do arquivo de dados (`RESOURCE.001`) **é lido de dentro** do
> `RESOURCE.MAP` (campo de 13 bytes), não é fixo no código (`resource.c:358`). Isso
> permite múltiplos volumes (`RESOURCE.002`…) no formato da família, embora o JC use
> apenas um.

---

## 3. `RESOURCE.MAP` — o índice

Layout lido por `parseMapFile()` (`jc_reborn/resource.c:342`); confirmado em
`castaway/docs/resindex.md`:

```
offset  tam  campo
0       4    "salt" / desconhecido (4 bytes; na família DGDS são seeds do hash)
4       2    numEntries (u16 LE)            ; nº de recursos
6       1    desconhecido
7       13   resFileName                    ; ASCIIZ, nome do arquivo de dados ("RESOURCE.001")
20      ...  numEntries × { u32 length; u32 offset }
             length = TAMANHO DESCOMPRIMIDO da entrada
             offset = posição da entrada dentro de RESOURCE.001
```

> O tamanho **comprimido** de cada entrada é obtido pela diferença entre offsets
> consecutivos (`resindex.md`).

### ⚠️ Divergência JC × família DGDS (crucial)
No **Rise of the Dragon / Heart of China** (ScummVM), o índice (`VOLUME.RMF`/`.VGA`)
é estruturado como **multi-volume com hash**:
```
salt[4], u16 nvolumes, então por volume:
   nome[13], u16 nfiles, nfiles × { int32 hash; u32 offset }
```
onde `hash = dgdsHash(nomeDoArquivo, salt)` e o nome é resolvido por hash. **No Johnny
Castaway, a primeira `u32` de cada entrada é o _comprimento descomprimido_, não um
hash** — e o nome do recurso é lido de dentro do `RESOURCE.001` (§4). Ou seja: para o
Wilson Reborn, **use o parser do `jc_reborn`/`castaway`** (length+offset), não o do
ScummVM. A função `dgdsHash()` (com overflow `int16` proposital) é relevante só se um
dia quisermos suportar os outros jogos DGDS.

---

## 4. `RESOURCE.001` — o arquivo de recursos

Para cada entrada do índice, `parseResourceFile()` (`resource.c:373`) faz
`fseek(offset)` e lê:
```
13   resName (ASCIIZ)        ; os ÚLTIMOS 4 chars dão o tipo: ".ADS" ".BMP" ".PAL" ".SCR" ".TTM"
4    resSize (u32 LE)
...  dados em chunks (ver §5)
```
O tipo determina o parser. Limites no jc_reborn: `MAX_ADS 100, BMP 200, PAL 1, SCR 20,
TTM 100`. Há exatamente **1 paleta global** (`palResources[0]`), usada em tudo.

`FILES.VIN` aparece como recurso `.VIN` mas é **ignorado** (é só uma listagem de
arquivos) — `resource.c:427`.

---

## 5. Container de chunks

Cada recurso é uma sequência de **chunks** com tag ASCII. Header (lido por
`DgdsChunk::readHeader`, `dgds.cpp:371`):
```
4    id      ; 3 letras + ':'  (o 4º byte DEVE ser ':' = 0x3A, senão parse inválido)
4    size (u32 LE)
```
- **Bit de container:** se `size & 0x80000000`, o chunk é um **container** (aninhado) e
  **não tem corpo próprio** — os chunks seguintes são seus filhos. Limpe o bit:
  `size &= ~0x80000000`.
- **Chunk "packed" (comprimido):** alguns chunks (conforme tipo de arquivo) têm o corpo
  comprimido, prefixado por:
  ```
  1    compressionMethod   ; 0=None, 1=RLE, 2=LZW  (3=RLE2 em alguns; ver §6)
  4    unpackSize (u32 LE)  ; tamanho descomprimido
  ...  corpo comprimido     ; tamanho = size - 5
  ```

**Tags de chunk relevantes** (lista completa nas notas do ScummVM):
`VER:` versão · `RES:` lista numerada de nomes de `.TTM` (em `.ADS`) · `SCR:` bytecode
ADS **ou** imagem de tela · `TT3:` bytecode TTM · `TAG:` tabela id→string · `PAG:`
contagem de páginas (TTM) · `TTI:` contagem de instruções (TTM) · `INF:` cabeçalho
(dimensões de imagem / índices) · `BIN:` plano de pixels (nibble baixo) · `VGA:` plano
de pixels (nibble alto) **ou** paleta · `DIM:` dimensões (SCR) · `MA8:` pixels 8bpp
(256 cores, Heart of China) · `MTX:` tilemap · `SNG:` música · `FNT:` fonte.

---

## 6. Decompressão

Dispatcher por `compressionMethod`:

### 6.1 Método 0 — None
Cópia direta.

### 6.2 Método 1 — RLE (`uncompressRLE`, `uncompress.c:180`)
Dirigido por byte de controle:
- byte `0x80` → **no-op** (escreve nada);
- bit alto **set** (`& 0x80`) → **repetição**: `count = control & 0x7F`; lê 1 byte e o
  repete `count` vezes;
- bit alto **clear** → **literal**: copia os próximos `control` bytes verbatim.

### 6.3 Método 2 — LZW (`uncompressLZW`, `uncompress.c:77`)
LZW de largura variável, **bits em ordem LSB-first**:
- início em **9 bits**, cresce até **12 bits** (máx. **4096** códigos);
- `free_entry` começa em **257** (256 = `0x100` reservado);
- **código `0x100` = CLEAR/reset**: realinha o stream de bits para o próximo limite de
  grupo, volta a 9 bits e `free_entry=256`;
- trata o caso clássico "KwKwK" (código ainda não na tabela).

> **Nuances que DEVEM ser replicadas exatamente** (do ScummVM): a contabilidade de
> alinhamento de bits (`_cacheBits`) no momento do CLEAR é um detalhe específico da
> Dynamix. O ScummVM aloca tabela de 16384 entradas, mas a largura de 12 bits limita a
> 4096 códigos efetivos — o valor que importa é **9→12 bits / 0x100=clear /
> 0x101=primeiro código livre / LSB-first**.

### 6.4 Método 3 — RLE2 (apenas JCOS)
O JCOS (`Compression.cs`) menciona um método **3 = RLE2**. Nem o jc_reborn nem o
ScummVM o implementam, e os ports JS lançam erro nele — provavelmente **não usado** pelo
Johnny Castaway. Tratar como "não suportado / investigar se aparecer".

---

## 7. Formatos por tipo de recurso

### 7.1 `.ADS` (script de sequência) — `parseAdsResource` (`resource.c:54`)
```
VER: + size + versão (5 bytes)
ADS: + 4 bytes desconhecidos
RES: + size + u16 numRes + numRes×{ u16 id; char nome[≤40] }   ; mapeia slot→arquivo .TTM
SCR: + bloco de bytecode (PACKED: método + unpackSize + corpo)  ; o script ADS
TAG: + size + u16 numTags + numTags×{ u16 id; char desc[≤40] }  ; nomes das sequências
```
O bytecode ADS está em [04-opcodes](04-engine-scripting-opcodes.md).

### 7.2 `.TTM` (script de animação) — `parseTtmResource` (`resource.c:269`)
```
VER: + versão
PAG: + u32 numPages + 2 desconhecidos
TT3: + bloco de bytecode (PACKED)                ; a animação
TTI: + 4 desconhecidos
TAG: + numTags×{ u16 id; char desc[≤40] }        ; "cenas" (pontos de entrada) dentro do TTM
```

### 7.3 `.BMP` (folha de sprites) — `parseBmpResource` (`resource.c:134`)
```
BMP: + u16 width, height
INF: + size + u16 numImages + u16 widths[numImages] + u16 heights[numImages]
BIN: + bloco de pixels comprimido (4-bpp, 2 pixels/byte)
```
É uma **spritesheet**: um stream concatenado de pixels, fatiado por imagem segundo as
larguras/alturas. Pixels são **4 bits** (16 cores) — 2 pixels por byte (nibble alto
primeiro).

### 7.4 `.SCR` (imagem de tela cheia) — `parseScrResource` (`resource.c:222`)
```
SCR: + totalSize + flags
DIM: + size + u16 width, height
BIN: + imagem 4-bpp de tela cheia (comprimida)
```

### 7.5 `.PAL` (paleta) — `parsePalResource` (`resource.c:183`)
```
PAL: + size + 2 desconhecidos
VGA: + 4 bytes
256 × { r, g, b }   ; valores VGA de 6 bits (0..63)
```
Apenas as **primeiras 16 cores** são usadas (JC é 16 cores). Conversão para 8 bits:
`componente << 2`. **Atenção à ordem de armazenamento** no jc_reborn: ele guarda como
BGR (`[0]=b<<2, [1]=g<<2, [2]=r<<2`).

### 7.6 Imagens em jogos de 256 cores (família DGDS)
JC é 16 cores (só `BIN:`). Nos jogos maiores, a cor vem de **dois planos de 4 bits**:
`VGA:` (nibble alto) + `BIN:` (nibble baixo), recombinados 2 pixels por par de bytes
(`convertBitmap`, `dgds.cpp:501`). `MA8:` = 8bpp direto (Heart of China). Útil saber se
o Wilson Reborn for suportar outros jogos DGDS no futuro.

---

## 8. Dados que **não** estão no `RESOURCE.001`

A **tabela de animação de caminhada** do Johnny (frames de walk + bookmarks) **não está
nos recursos** — foi extraída do **executável `SCRANTIC.SCR`** pelo utilitário
`extract_walk_data.c` (lê triplas a partir do offset `0x188ea` até `0x019456`, com
`flip = word0>>15`, `spriteNo = word0 & 0x7fff`). Isso gerou o `walk_data.h`.

Da mesma forma, `story_data.h` (tabela de cenas/dias) e `calcpath_data.h` (matriz de
adjacência do pathfinding) **codificam a intenção do designer que não pode ser
recuperada dos dados** — foram reconstruídos por observação. Ver
[05-arquitetura](05-arquitetura-do-engine.md) §walk e §pathfinding.

> **Consequência para o Wilson Reborn:** esses 3 conjuntos de dados (`story_data.h`,
> `walk_data.h`, `calcpath_data.h`) precisam ser **portados verbatim** (ou
> re-extraídos do `SCRANTIC.SCR`), pois não vêm do `RESOURCE.001`.

---

## 9. Sons

Os 24 efeitos são carregados como **`.wav` externos** (`sound0.wav`…`sound24.wav`,
faltando alguns índices como 11 e 13) pelo jc_reborn/JCOS — eles foram **extraídos** por
Hans Milling (JCOS). No original, os efeitos do SCRANTIC ficavam no formato **`.SX`** da
família DGDS (container com chunks `INF:`/`TAG:`/`DAT:`, PCM marcado por `0x00FE`). O
jc_reborn toca um mixer de software de 1 canal; `sound0` é o cue genérico de transição
de cena de enredo. Detalhes de música MIDI (`.SNG`) e PCM estão nas notas do ScummVM
(§9), mas o **JC praticamente só usa efeitos PCM curtos**.

---

## 10. Resumo: o mínimo para ler tudo do Johnny

1. Parsear `RESOURCE.MAP` (length+offset, formato JC — §3).
2. Para cada entrada em `RESOURCE.001`: ler nome+size, então os chunks (§5).
3. Implementar **RLE** e **LZW** (§6).
4. Parsers de `.ADS`, `.TTM`, `.BMP`, `.SCR`, `.PAL` (§7).
5. Portar/extrair `story_data.h`, `walk_data.h`, `calcpath_data.h` (§8).
6. Interpretar bytecode TTM/ADS ([04](04-engine-scripting-opcodes.md)) sob a arquitetura
   do engine ([05](05-arquitetura-do-engine.md)).
