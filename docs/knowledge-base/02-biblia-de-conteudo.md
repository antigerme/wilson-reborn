# 02 — Bíblia de Conteúdo (TODOS os comportamentos, gags, eventos e história)

> **Objetivo deste documento:** registrar *exaustivamente* tudo o que o Johnny faz
> no original, para que o **Wilson Reborn não perca nenhum recurso**. É o "contrato
> de paridade" com o original: eventos, sequências narrativas, gags, easter eggs,
> datas comemorativas, comportamentos e brincadeiras.
>
> Fontes: catálogo canônico em https://johnny-castaway.com/ + os dados de cena
> decodificados em `repos/jc_reborn/story_data.h`, `repos/jc_reborn/story.c` e
> `repos/castaway/src/scrantic/metadata/scenes.mjs`.
>
> **Como ler:** quando conhecido, cada comportamento aponta o **arquivo `.ADS` +
> tag** que o implementa (ver §13). Isso liga o "o quê" (este doc) ao "como"
> ([04-opcodes](04-engine-scripting-opcodes.md) e [05-arquitetura](05-arquitetura-do-engine.md)).

---

## 1. O cenário (o "mundo" do Johnny)

- **Ilha minúscula** com **um coqueiro** (palmeira). É o palco fixo.
- **Mar** ao redor, com ondas animadas; **nuvens** desenhadas em posições aleatórias.
- **Maré**: existe **maré baixa** (*low tide*), que expõe mais areia e habilita
  certas cenas (peixe à meia-luz, etc.). Aleatória quando a cena permite (`LOWTIDE_OK`).
- **Ciclo dia/noite**: no original é um **ciclo de 8 horas** (não 24h). É "noite"
  nas bordas de cada bloco de 8h. Há **pesca/mergulho ao luar na maré baixa**.
- **Posição da ilha**: redesenhada em **posição aleatória** na tela a cada sequência
  (quando a cena permite `VARPOS_OK`), com faixas de coordenadas específicas.
- **A jangada (raft)** cresce ao longo da história (ver §2 e §3). É elemento de
  cenário *e* de enredo.
- **Itens de feriado** (árvore de Natal, abóbora, trevos, faixa de Ano Novo)
  aparecem desenhados na ilha nas datas certas (ver §9).

---

## 2. O arco narrativo de 11 dias (a "história")

O screensaver avança **um "dia de história" cada vez que a data real do sistema
muda** (uma vez por dia real). O ciclo tem **11 dias** e então recomeça
(`story.c`: `currentDay` vai de 1 a 11, depois reinicia). Personagens centrais da
narrativa: **Mary, a sereia** e **Suzy, a moça da cidade**.

| Dia | Evento narrativo | Cena (ADS#tag) |
|---:|---|---|
| **1** | A **sereia** observa o Johnny enquanto ele pesca (estabelece a presença dela). | `MARY.ADS#2` |
| **2** | Johnny escreve um **SOS e joga numa garrafa**; no balão de pensamento aparece um **mini-Johnny** parado na ilha. | `JOHNNY.ADS#2` |
| **3** | A garrafa chega a **Suzy**, na cidade, que imagina receber a visita de um Johnny idealizado. A **jangada cresce**. Um **tubarão** tenta atacá-lo durante o banho. | `SUZY.ADS#1` |
| **4** | Johnny encontra a **sereia** e a convida para um encontro, presenteando-a com um **colar de conchas**. Jangada quase pronta. | `MARY.ADS#3` |
| **5** | À noite, Johnny e a sereia **jantam e dançam**. Ele veste **traje de gala (fraque e cartola)** — mas continua descalço. | `MARY.ADS#1` |
| **6** | Johnny **desenha a si mesmo abraçado com Suzy** e registra a fantasia em outra **garrafa**. | `JOHNNY.ADS#3` |
| **7** | A sereia reaparece; Johnny a convida para voltar ao continente com ele. Ela **vai embora chorando**, recusando. | `MARY.ADS#4` |
| **8** | Johnny se **despede do tubarão e da sereia** e **parte na jangada** completa (rema para longe). | `MARY.ADS#5` *(LEFT_ISLAND, NORAFT)* |
| **9** | Um **relógio (sapo/"frog clock")** aparece; a jangada flutua; ele chega à praia e **se reencontra com Suzy**. | `SUZY.ADS#2` |
| **10** | Johnny **dorme na mesa de um escritório**, sonhando com a ilha (e com a sereia). | `JOHNNY.ADS#6` |
| **11** | Johnny **volta à ilha de avião** (paraquedas), reiniciando o ciclo. | `JOHNNY.ADS#1` |

> **Observação de pesquisa:** a Wikipedia menciona um arco de "~120 dias"; os engines
> open-source implementam **11 dias** (dado de `story_data.h`). Provável diferença
> entre a percepção do público e a estrutura real dos dados. **Wilson Reborn deve
> seguir os 11 dias dos dados**, mas isso é configurável.

### Crescimento da jangada (lógica de `story.c`)
| Dia da história | Estágio da jangada |
|---:|---:|
| 0–2 | 1 (início) |
| 3 | 2 |
| 4 | 3 |
| 5 | 4 |
| 6+ | 5 (completa) |

Cenas com flag `NORAFT` forçam a jangada a **não** aparecer (ex.: dia 8, quando ele
já partiu nela).

---

## 3. Atividades do dia a dia e gags (comportamentos "comuns")

Estas são as cenas "mundanas" e os gags que rodam aleatoriamente entre os eventos de
enredo. O escalonador toca de **6 a 20 cenas intermediárias** caminhando entre
"spots" da ilha, e então uma cena "final" (`story.c`).

### 3.1 Pesca (`FISHING.ADS`, e gags em `ACTIVITY/MISCGAG`)
Principal fonte de comida; há **pesca ao luar na maré baixa**.
- **Capturas comuns:** bota velha (às vezes guardada atrás da árvore); caranguejo
  (morde o nariz do Johnny, que o joga de volta); estrela-do-mar (descartada).
- **Capturas menos comuns** (guardadas atrás da árvore): boia marcada **"SS Titanic"**
  (causa um *glitch* gráfico acima da cabeça quando ele pesca pela esquerda); peixe
  verde; tábua de madeira; pequeno polvo.
- **Eventos raros:**
  - **Tubarão:** Johnny fisga um tubarão e acaba fazendo **esqui aquático** atrás dele.
  - **Polvo grande:** pesca vários peixes e então um **polvão** que o persegue até a
    árvore, **rouba todos os peixes** e mergulha de volta, deixando-o furioso.
  - **Peixe verde:** ocasionalmente **esguicha água** nele, que o joga de volta com
    nojo.
- **Detalhe de animação:** ambidestria — pescando à **direita** da ilha usa molinete
  para destros; à **esquerda**, molinete para canhotos.

### 3.2 Natação, mergulho e banho (`swimming` / `ACTIVITY.ADS`)
- **Mergulho do coqueiro:** sobe na palmeira e mergulha no mar (às vezes ao luar, na
  maré baixa). **Mergulhos com nota:** uma **estrela-do-mar, um caranguejo, um peixe
  e uma gaivota** seguram cartões dando notas. O **caranguejo é maluco**: dá **−0.5**
  para um bom mergulho e **10!** para uma barrigada ruim. *(cenas "MUNDANE DIVE",
  "GAG DIVES")*
- **Banho no mar:** Johnny se lava sentado no mar; ao perceber que está sendo
  observado, **se cobre como pode**, vai se vestir atrás da árvore e depois **sacode
  o punho** (bravo). A **gaivota às vezes rouba a sunga** (ver §6.3). *(cena "JOHN BATH")*
- **Ataque do tubarão (gag):** Johnny chega à beirada com **toalha e escova de
  banho**, testa a água com o dedão — o **tubarão pula e o morde**; ele cai
  ensanguentado contra a palmeira, larga toalha/escova. Ao se inspecionar, descobre
  que a perna está **intacta** (estava só sentado sobre ela) e fica aliviado.

### 3.3 Leitura (`reading` / `ACTIVITY.ADS`: "GULL READING", "JOHN READ")
- **Leitura confusa:** o gag mais frequente — ele segura o livro **de cabeça para
  baixo**, e mesmo "do jeito certo" não entende; **vira as páginas da esquerda para a
  direita** (fãs brincaram que o livro estaria "em hebraico ou árabe").
- **Cochilo + coco:** ele lê, vai **cochilando e acordando** com solavancos da cabeça;
  os solavancos **sacodem o coqueiro** até um **coco cair na sua cabeça**.
- **Gaivota ladra de livro:** a gaivota mergulha e **rouba o livro** (ver §6).

### 3.4 Dormir (`common#sleeping`)
- Johnny **tira sonecas** com frequência (começa a **roncar** quase imediatamente).
- **Piratas o amarram enquanto dorme** (ver §7) — e, nesse caso específico, ele **não
  ronca**.

### 3.5 Fazer fogo e cozinhar (`common`)
- Tenta acender fogo **esfregando gravetos**: consegue após **2–4 tentativas** ou
  desiste — e aí o fogo acende "**espontaneamente**".
- Com o fogo, **cozinha a pesca** (muitas vezes o peixe verde, ou a **bota velha**
  quando está com muita fome). Ao comer um **pequeno polvo**, este **gruda no rosto**
  dele antes de ser comido.

### 3.6 Comer cocos (`common#coco`)
- Cocos caem da árvore com **padrões de quique variados** (forte para a direita; ou
  dois quiques fracos para a esquerda).
- Em uma variação, a **cabeça do Johnny gira completamente** (fã brincou que ele
  estaria na "Ilha do Diabo").
- Ao conseguir o coco, ele o **bate na árvore** para quebrar a casca, senta e come.

### 3.7 Construir a jangada (`common#raft` / `BUILDING.ADS`)
- Constrói a **jangada** (peça de cenário e de enredo). Em sessões longas, a jangada
  às vezes **volta ao tamanho original** (também há bug de "jangada superconstruída"
  — ver §12).

### 3.8 SOS na garrafa (`common#bottle`)
- Escreve mensagens, põe em **garrafas** e atira ao mar. Em geral **voltam à praia**,
  às vezes **chegam a outro lugar** (a Suzy — ver §2).
- Pensa **"SOS"**; às vezes pensa numa **moça bonita**. No **dia 2**, o balão mostra
  um **mini-Johnny** parado na ilha.

### 3.9 Castelo de areia (`common#castle` / `BUILDING.ADS`)
- Johnny **constrói um castelo de areia** — isso **dispara a cena dos piratas
  "King Kong"** (ver §7.1).

### 3.10 Cooper / corrida (`common#jogging`)
- Johnny **faz cooper** (corre) pela ilha — uma das atividades de rotina.

### 3.11 Telescópio / luneta (`visitors#notseen`)
- Usa um **telescópio/luneta** para vasculhar o horizonte. O gag recorrente: enquanto
  ele olha numa direção, **algo passa às suas costas** sem que ele veja (ver §8.1).

### 3.12 Dança da chuva / "nativo" (`ACTIVITY.ADS`: "NATIVE 1/3")
- Com calor, Johnny se **veste de pajé/feiticeiro** e faz uma **dança da chuva**. Uma
  nuvem solta **uma única gota** e então ele **leva um raio**. (Também aparece em
  cenas com turistas — ver §8.2.)

---

## 4. Mary, a sereia (`MARY.ADS`)
A sereia se chama **Mary**. Interações (algumas são os beats de enredo dos dias
1/4/5/7/8 — ver §2):
- **Ouve mas não vê:** Johnny pesca, a sereia se aproxima por trás; ele ouve o
  barulho d'água, se reposiciona e fisga algo pesado — **dentadura postiça** ou **bota
  velha**.
- **O convite:** lendo, ele a vê nadando perto; ela lhe dá um **colar de conchas**, ele
  oferece a **boia do Titanic**; pensam em jantar (ela imagina um **sinal de trânsito
  verde**), depois se despedem.
- **O jantar:** ele troca de roupa atrás da árvore (**cartola e fraque**), monta toda
  uma **mesa de jantar**, eles comem; depois ele traz um **gramofone** e **dançam**
  até ela voltar ao mar.
- **A súplica:** Johnny fica em pé na jangada **implorando** que ela embarque; ela vai
  embora e ele fica **arrasado**. (Variação: ela pergunta para que serve a jangada,
  ele mostra visões da cidade, e ao descobrir que ele quer ir embora, **ela chora**.)
- **A partida:** ele tenta convencê-la a ir na jangada; ela fica com o **tubarão**
  (estão **rindo**) e Johnny parte sozinho.
- **Devaneio:** Johnny dorme na mesa do escritório e **sonha** que janta com a sereia
  na ilha (dia 10).

---

## 5. Suzy, a moça da cidade (`SUZY.ADS`) e as fugas (`leaving`)
- **A garrafa e Suzy:** **Suzy** (moça da cidade), tomando sol de **biquíni rosa** num
  resort, acha a garrafa do Johnny e o **imagina como um homem atraente** varrendo-a
  dos pés (no devaneio dela, ela aparece mais jovem e magra). Há o **inverso**: Johnny
  acha a mensagem de Suzy e devaneia com ela olhando seu relógio.
- **Partida na jangada:** "Johnny acabou de subir na jangada e remou para longe",
  levando **remo e saco**. Um **golfinho** (depois identificado como **tubarão**) e a
  **sereia** acompanham antes de ele voltar às atividades normais.
- **Encontro no resort:** Johnny passa de jangada por Suzy perto de um resort com
  arranha-céus; ela o **agarra e beija** apaixonadamente.
- **Puxão de orelha:** o clima azeda quando Suzy descobre **chiclete no decote** depois
  do beijo; furiosa, ela briga com ele e **puxa sua orelha**.
- **Cena íntima (não confirmada):** vários relatos de "**naughty things**" entre Johnny
  e uma mulher, com confusão se era num ambiente interno ou só uma tela menor.

---

## 6. A gaivota (`seagull`)
A gaivota quase sempre leva a melhor; "Johnny usually comes off worse".
- **Ladra de livro:** leva o livro ao topo da palmeira e "**lê**", virando páginas com
  o bico.
- **Senta na cabeça:** rouba o livro e pousa na cabeça do Johnny; ele tenta tirá-la com
  um **porrete**, mas **acerta a si mesmo** (cria um galo) — a gaivota fica pairando.
- **Ladra de roupa:** enquanto ele toma banho no mar, ela mergulha e **rouba a sunga**
  (duas variações de imagem).
- **Ninho no chapéu:** pousa na cabeça, **rouba o chapéu**, leva ao topo da árvore e
  **faz um ninho** nele.
- **Ninho no peito:** depois que os piratas amarram o Johnny, a gaivota faz um **ninho
  no peito dele, põe um ovo** e vai embora (ver §7.2).

---

## 7. Piratas (`pirates`)
### 7.1 Cena "King Kong" (disparada ao construir o castelo de areia)
Quando Johnny constrói um **castelo de areia**, chega um **galeão pirata em
miniatura**. Piratinhas remam até a praia, **ocupam o castelo**, hasteam **bandeira** e
**disparam canhões** contra o Johnny. Ele se refugia na palmeira enquanto **vários
biplanos minúsculos** decolam do castelo para atacá-lo. A sequência termina com Johnny
**caindo na água** — paródia de **King Kong (1933)** no topo do Empire State.

### 7.2 Cena "Viagens de Gulliver" (enquanto dorme)
Piratas se aproximam do Johnny **dormindo** e o **amarram com cordas** — referência a
*Gulliver* de Jonathan Swift (o site sugere que seriam uma "Marinha de Liliput"). É
**noturna**; nela o Johnny **não ronca** e a gaivota pode **não aparecer**; quando
aparece, faz **ninho no peito** dele e **põe um ovo**. Há um **bug** no fim da cena
(retângulo estranho no mar — ver §12).

---

## 8. Visitantes e tentativas de resgate (`visitors`)
### 8.1 Passam por trás dele (ele quase nunca vê — `notseen`)
- **Lancha** com uma **mulher e um cachorro**.
- **Biplano** passa enquanto ele usa o telescópio.
- **Helicóptero** (em 28/12/1998 um fã sugeriu ser um **autogiro**, não helicóptero).
- **Avião** voando baixo sobre a ilha.

### 8.2 Visitantes que ele vê
- **Barco de festa:** chega um barco com **foliões** que o levam a bordo; ele **nada de
  volta** à ilha e o barco vai embora — bem quando Johnny percebe o que fez. Uma mulher
  faz **esqui aquático** atrás de uma lancha e **derruba o Johnny**.
- **Johnny pelado (3 variações):** (1) um casal chega e ele implora para ser levado,
  tirando **toda a roupa** para convencer a moça; (2) ele dança de **traje tribal**,
  turistas o fotografam, e ele **rasga a roupa** e a balança no ar; (3) durante a
  **dança da chuva**, turistas o confundem com nativo e, para provar que não é, ele
  **tira a roupa** — o que irrita o homem.
- **Johnny "Terminator":** desta vez ele **realmente avista** o avião; joga um **coco**
  para chamar atenção do piloto, mas **acerta o avião**, que **cai no mar**. O piloto
  salta de **paraquedas** antes do impacto.
- **Johnny vândalo (não confirmado):** atira um coco num navio tentando afundá-lo.
- **Navio gigante (cargo):** Johnny avista um navio ao longe e **pula para chamar
  atenção**; o navio se revela **enorme** e quase **corta a ilha ao meio** — Johnny
  corre para se salvar. *(cena `VISITOR.ADS#3`, marcada `HOLIDAY_NOK`: nunca mostra
  itens de feriado, senão seriam desenhados sobre o casco que toma a tela.)*
- **Sereia:** ver §4.

---

## 9. Datas comemorativas / feriados (`annivers` + lógica de `story.c`)
Itens especiais são desenhados na ilha em faixas de datas (comparação de string
`"MMDD"` em `story.c`). **Pode-se forçar ajustando o relógio do sistema.**

| Feriado | Faixa de datas (engine) | O que aparece / acontece | `holiday=` |
|---|---|---|---:|
| **Ano Novo** | **29/12 → 01/01** | Faixa **"Happy New-Year"** na palmeira. | 4 |
| **Dia de São Patrício** | **15/03 → 17/03** | Ilha coberta de **trevos de 4 folhas** (a intenção eram trevos/shamrocks). | 2 |
| **Halloween** | **29/10 → 31/10** | Grande **abóbora (jack-o'-lantern)** na frente da ilha. | 1 |
| **Natal** | **23/12 → 25/12** | **Árvore de Natal** na ilha. Variação: ao pescar o **polvão**, este **rouba bolas de Natal** da árvore antes de mergulhar. | 3 |

> **Independence Day (4 de julho):** a Wikipedia cita o 4 de Julho entre os feriados,
> mas **não está** no `annivers` do site nem implementado no `jc_reborn`. **Item em
> aberto** — investigar nos dados originais; pode existir arte/cena não escalonada.
> Wilson Reborn deve deixar a tabela de feriados **extensível** (a roadmap do castaway
> sugere "extend festive days").

---

## 10. Eventos raros e easter eggs (`unusual`)
- **Briga (Johnny fantasma):** um Johnny **transparente** sai da água enquanto o Johnny
  normal persegue um coco; eles **brigam** e o Johnny #1 derruba o fantasma de volta no
  mar. Em seguida entra um avião, ele joga o coco, derruba o avião e o piloto salta.
- **Bolas de prata:** **duas bolas/tigelas prateadas**, uma de cada lado da palmeira,
  antes da cena terminar abruptamente.
- **Relógio em tempo real:** um **relógio no balão de pensamento** mostra a **hora real
  do computador** (relatos de 1997 e 2008).
- **Dança da chuva:** ver §3.12 (nuvem → uma gota → raio).
- **"Feeding the Fishes":** um **tubarão pula na ilha**, **engole o Johnny**, nada por
  aí, faz careta e o **cospe de volta**.
- **Johnny derretendo:** ele usa um **leque amarelo**, os joelhos amolecem e ele
  **derrete num blob** (também acontece ao luar).
- **Devaneio no escritório:** depois de um relógio aparecer, Johnny aparece num
  **escritório sonhando** com a ilha e a sereia (dia 10 — ver §2).
- **"Home Again?" / THE END:** uma **telinha silhuetada** mostra um avião sobre a ilha,
  um homem de **paraquedas** pousando, pulando de alegria, e o texto **"THE END"**.
- **Padrão de perambulação:** Johnny perambula ~5–6 min e então "faz a coisa especial"
  antes de a cena trocar.

---

## 11. Sons
O original tem **24 efeitos sonoros** (`sound0.wav`…`sound24.wav`, com lacunas em 11 e
13). O `sound0` é tocado em transições de cena de enredo (`story.c`: `soundPlay(0)`
quando a cena tem `dayNo`). MD5/tamanhos exatos em
[01-história](01-historia-e-creditos.md) / `repos/jc_reborn/README.md`.

---

## 12. Bugs originais (catalogados em `bugs`)
Importante decidir, no Wilson Reborn, **quais são "charme" a preservar** e **quais
corrigir**. Lista do site:

**Instalação:** "Can't find data files" no Win2000/NT (procura em `windows`, instala em
`winnt`).

**Glitches visuais:** congelar na tela-título (Win95); **Johnny sumido** (pouca
memória — só a vara e o som); **mancha preta** semicircular ao puxar pesca pela
esquerda; **retângulo** no mar após os piratas; **vara some** ao virar para a palmeira
com a bota; **jangada superconstruída**; **palmeira transparente**; **nuvem com
linhas**; **Johnny voador** (duplicata suspensa após o mergulho); **dançarinos
fantasmas** nas nuvens após o raio; **Johnny na caixa** (aparece no quadrado que
deveria embaralhar após o navio pirata); **ilha gigante** / **múltiplas ilhas** /
**dezenas de Johnnys** (após execução longa); **caixas pretas** (às vezes travam);
**nuvem escura**; **mar vermelho**; **cena dia+noite simultânea**; **gêmeos** (Johnny
duplicado, ex.: na cena "terminator"); **"tidy your room"** (mesa/gramofone ficam no
fundo após o jantar com a sereia); **divisão de cor da tela**; **congelar subindo a
árvore**; **sem Johnny**; **Johnny teleportando**; **coco escondido**.

**Áudio:** **só som** sem vídeo (pode persistir no desktop); **"muttering mode"**
(placa de som trava resmungando após crash).

> Muitos desses bugs são artefatos do Windows 3.1/16-bit e **desaparecem
> naturalmente** num engine moderno. Alguns são **gags acidentais queridos** pela
> comunidade (ex.: "ilha gigante", "dezenas de Johnnys") — poderiam virar um
> **modo/easter egg opcional** no Wilson Reborn.

---

## 13. Mapa Cena→Comportamento (os 10 arquivos `.ADS`)

O `story_data.h` do `jc_reborn` define **63 cenas** distribuídas em **10 arquivos
`.ADS`** (cada arquivo agrupa "tags" numeradas; cada tag é uma cena). Mapeamento
geral, com os nomes descritivos confirmados em `castaway/.../scenes.mjs` quando
disponíveis:

| Arquivo `.ADS` | Conteúdo (categoria) | Cenas/Tags notáveis |
|---|---|---|
| **ACTIVITY.ADS** | Atividades/gags variados | #1 *GAG DIVES*, #4 *MUNDANE DIVE*, #6 *GAG JOHN READ*, #7 *MUNDANE JOHN READ*, #8 *JOHN BATH*, #10 *GULL 1 READING*, #11 *GULL 2 BATHING*, #12 *GULL 3 STILL READING*, #5 *NATIVE 1*, #9 *NATIVE 3* |
| **BUILDING.ADS** | Construção (jangada / castelo de areia) | tags 1–7 |
| **FISHING.ADS** | Pesca (capturas, lados esquerdo/direito) | tags 1–8 (#4,#7,#8 marcadas `LEFT_ISLAND`) |
| **JOHNNY.ADS** | Beats de enredo do Johnny | #1 → dia 11 (volta de avião), #2 → dia 2 (SOS), #3 → dia 6 (desenho Suzy), #6 → dia 10 (escritório), #4/#5 livres |
| **MARY.ADS** | A sereia Mary | #2 → dia 1, #3 → dia 4, #1 → dia 5, #4 → dia 7, #5 → dia 8 (parte na jangada) |
| **MISCGAG.ADS** | Gags diversos | tags 1–2 |
| **STAND.ADS** | Poses/ocioso em cada spot da ilha | tags 1–16 (transições/idle) |
| **SUZY.ADS** | A moça da cidade Suzy | #1 → dia 3, #2 → dia 9 |
| **VISITOR.ADS** | Visitantes/resgates | #1, #3 (cargo gigante, `HOLIDAY_NOK`), #4, #5 (`LEFT_ISLAND`), #6, #7 |
| **WALKSTUF.ADS** | "Coisas" ligadas a caminhar | tags 1–3 |

> Os "spots" (A–F) e "headings" (S, SW, W, NW, N, NE, E, SE) definem **onde** na ilha a
> cena começa/termina e para **onde** o Johnny olha, permitindo que o engine **caminhe**
> transicionalmente entre cenas. Detalhes do modelo em
> [05-arquitetura-do-engine](05-arquitetura-do-engine.md).

---

## 14. Checklist de paridade (resumo "não perder nada")

- [ ] Arco de **11 dias** (Mary + Suzy), com avanço por data real e reinício.
- [ ] **Pesca** (todas as capturas comuns/raras + ambidestria + polvão + tubarão-esqui).
- [ ] **Natação/mergulho** com **júri de bichos** e o caranguejo de notas invertidas.
- [ ] **Banho** + gaivota ladra de sunga + susto do tubarão (perna "intacta").
- [ ] **Leitura** (livro de cabeça pra baixo, cochilo→coco, gaivota leitora).
- [ ] **Dormir/roncar** + amarração pelos piratas.
- [ ] **Fogo/cozinhar** (2–4 tentativas, polvo no rosto).
- [ ] **Cocos** (quiques, cabeça girando, quebrar na árvore).
- [ ] **Jangada** (5 estágios) e **SOS na garrafa** (mini-Johnny no dia 2).
- [ ] **Castelo de areia** → **piratas King Kong**.
- [ ] **Cooper** e **telescópio** (algo passando por trás).
- [ ] **Dança da chuva** (gota → raio).
- [ ] **Mary, a sereia** (todas as 6 interações).
- [ ] **Suzy** + cenas de **fuga/resort/beijo/puxão de orelha**.
- [ ] **Gaivota** (5 gags).
- [ ] **Piratas** (King Kong + Gulliver, com ninho/ovo no peito).
- [ ] **Visitantes** (lancha+mulher+cão, biplano, helicóptero/autogiro, avião baixo,
      barco de festa, esquiadora, turistas, terminator, navio gigante, pelado x3).
- [ ] **4 feriados** (Ano Novo, S. Patrício, Halloween, Natal) + tabela extensível.
- [ ] **Easter eggs raros** (Johnny fantasma, bolas de prata, relógio real, derreter,
      "feeding the fishes", "THE END/Home Again").
- [ ] **24 sons**.
- [ ] **Maré baixa**, **ciclo dia/noite**, **posição aleatória da ilha**, **nuvens**.

---

### Fontes
johnny-castaway.com (common, fishing, swimming, reading, mermaid, pirates, seagull,
visitors, leaving, annivers, story, unusual, bugs); `repos/jc_reborn/story_data.h` e
`story.c`; `repos/castaway/src/scrantic/metadata/scenes.mjs` e `types.mjs`.
