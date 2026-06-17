# 01 — História, Créditos e Linhagem Open-Source

> Parte da Base de Conhecimento do **Wilson Reborn** — clone moderno e portável do
> screensaver *Johnny Castaway*. Este documento estabelece o contexto histórico,
> os créditos do original e a "árvore genealógica" dos projetos open-source que
> servem de referência.

---

## 1. O que é Johnny Castaway

*Johnny Castaway* (nome comercial completo: **"Screen Antics™: Johnny Castaway"**)
foi lançado em **novembro de 1992** para **Windows 3.1**. Foi anunciado como
**"the world's first story-telling screen saver"** (o primeiro protetor de tela do
mundo que conta uma história).

Diferente dos protetores de tela da época (tubos, estrelas, toasters voadores), o
Johnny não era um *loop* aleatório: ele encena a vida de um náufrago numa ilha
minúscula com **um único coqueiro**, e uma **narrativa de fundo se revela
lentamente ao longo dos dias reais** — o screensaver lê o relógio/calendário do
sistema e avança a história conforme o tempo do mundo real passa.

O humor é de **sight gags** (piadas visuais) no estilo dos quadrinhos de
**Johnny Hart (B.C.)** e com a premissa cômica de náufrago à la *Gilligan's Island*:
todas as tentativas de resgate dão errado de formas absurdas.

- **Plataforma original:** Windows 3.1 (16-bit), requeria um 386SX. Distribuído em
  disquete de 3½".
- **Versão estável:** 1.02 (1993).
- **Modelo de negócio:** produto barato de produzir e muito lucrativo — citado por
  Ken Williams na mesma categoria de *The Incredible Machine* e *Hoyle Card Games*.

### Recepção
A *Computer Gaming World* chamou o lançamento de "a great launch" para a marca
Screen Antics e concluiu: *"Fans of Johnny Hart-style comics and sight gag lovers
everywhere should love it"*. O consenso é de uma novidade bem recebida que reforçou
a reputação da Sierra em software casual.

---

## 2. Créditos do original

| Papel | Pessoa | Observação |
|---|---|---|
| Produtor / idealizador | **Jeff Tunnell** | Fundador da Dynamix; criou a **Jeff Tunnell Productions (JTP)**, divisão da Dynamix |
| Design do personagem | **Shawn Bird** | Criou o visual "weathered but likable" do Johnny |
| Designer-chefe (lead designer) | **Chris Cole** | |
| Diretor de arte / gags | **Brian Hahn** | Responsável pelas piadas visuais |
| Animadora | **Sherry Wheeler** | Criou as animações |

**Cadeia de produção:** desenvolvido pela **Jeff Tunnell Productions** (divisão da
**Dynamix**), publicado pela **Sierra On-Line** sob a marca **Screen Antics**. Foi
um de três projetos iniciados em janeiro de 1992 pela JTP, junto com
*The Incredible Machine* e *Turbo Learning: Mega Math*.

---

## 3. O motor: DGDS / "SCRANTIC"

Internamente, o engine é o **DGDS — Dynamix Game Development System**, baseado em
tecnologia pré-existente da Sierra. A "personalidade" do screensaver é chamada
internamente de **SCRANTIC** (de *Screen Antics*) — daí o arquivo de fundo
`SCRANTIC.SCR` e o ícone `SCRANTIC.ICO`.

O **mesmo motor DGDS** roda outros jogos da Dynamix — fato importante porque a
documentação/decodificação do formato vale para todos eles:

- *Johnny Castaway Screen Saver*
- *Rise of the Dragon*
- *Heart of China*
- *The Adventures of Willy Beamish*
- *Quarky & Quaysoo's Turbo Science*

> Por isso o repositório `dgds-viewer` traz screenshots `dragon.png`, `willy.png`,
> `hoc.png`, `turbosci.png` e `dynamix.png`: ele é um visualizador genérico de
> recursos DGDS, não só do Johnny.

### Arquivos de dados originais (necessários para rodar qualquer clone)
- **`RESOURCE.MAP`** (1.461 bytes) — índice de recursos.
- **`RESOURCE.001`** (1.175.645 bytes) — todos os recursos comprimidos
  (animações, bitmaps, paletas, scripts).
- **`SCRANTIC.SCR`** — tela/imagem de fundo.
- Sons opcionais: `sound0.wav` … `sound24.wav` (24 efeitos; ver
  [02-bíblia-de-conteúdo](02-biblia-de-conteudo.md#11-sons)).

> MD5s e tamanhos exatos estão registrados no `repos/jc_reborn/README.md` e são
> reproduzidos em [03-formatos-de-dados](03-dados-originais-e-formatos.md).

### ⚖️ Nota legal (importante para o Wilson Reborn)
Os arquivos `RESOURCE.*` e os bitmaps/animações do Johnny são **propriedade
intelectual da Sierra/Dynamix** (hoje sob detentores dos direitos da Sierra). Todos
os clones open-source (JCOS, jc_reborn, castaway) **não distribuem** esses dados —
exigem que o usuário forneça os arquivos originais. **Estratégia recomendada para o
Wilson Reborn:** o engine é livre, mas (a) por padrão carrega os `RESOURCE.*`
originais que o usuário possui, e/ou (b) oferece um conjunto de assets recriados do
zero (arte nova) como pacote opcional para uma versão 100% redistribuível. Ver
[07-plano-do-port-moderno](07-plano-do-port-moderno.md).

---

## 4. A comunidade e a fonte canônica de comportamentos

O site **https://johnny-castaway.com/** é "the online source of all Johnny Castaway
since 1996". Originalmente mantido por **Maria Bare**; a administração foi
transferida para um novo curador em **4 de outubro de 2018**. É o **catálogo
canônico** de tudo que o Johnny faz — cada página documenta uma categoria de
comportamento, muitas vezes com relatos datados de fãs (1996–2008).

Páginas (todas capturadas na [bíblia de conteúdo](02-biblia-de-conteudo.md)):
`index/list` (índice A-Z), `common`, `fishing`, `swimming`, `reading`, `mermaid`,
`pirates`, `seagull`, `visitors`, `leaving` (fuga/partida), `annivers` (datas
comemorativas), `story`, `unusual` (raros/easter eggs), `bugs`.

---

## 5. Linhagem open-source (os projetos em `repos/`)

Cinco reimplementações independentes do **mesmo** motor — juntas formam a referência
técnica completa. A precedência histórica importa: cada projeto decifrou mais um
pedaço do formato.

| Projeto (pasta) | Autor | Linguagem / Stack | Papel como referência |
|---|---|---|---|
| **JCOS** — `Johnny-Castaway-Open-Source` | Hans Milling (*nivs1978*), 2015 | C# / WinForms (.NET) | **Pioneiro.** Primeiro a decodificar todos os arquivos de dados e a entender muitas instruções TTM/ADS. Também publica os `sound*.wav` extraídos. |
| **jc_reborn** | Jérémie Guillaume (*jno6809*), 2019 | C / SDL2 | **Mais completo como engine jogável.** Entende quase toda instrução TTM/ADS, implementa caminhada (walk) entre cenas, o escalonador aleatório de cenas, desenho da ilha/nuvens, ciclo de 11 dias e feriados. Roda em Linux e Windows (MinGW), 32/64-bit. **Melhor blueprint de gameplay.** |
| **castaway** | Alexandre Fontoura (*xesf*) | JavaScript (ES Modules), web (canvas) | Porte web; documenta o formato (`docs/resindex.md`), metadados de cena com **nomes descritivos** e uma *roadmap de melhorias* muito alinhada aos seus objetivos. |
| **dgds-viewer** | Alexandre Fontoura (*xesf*) | JS + React + Electron | Visualizador genérico de recursos DGDS (todos os 5 jogos). Interpretador de scripts mais elaborado (`process.js`). Bom para inspeção/depuração de assets. |
| **dgds** (ScummVM) | Vasco Costa (*vcosta*) e contribuidores | C++ (engine ScummVM) | **Autoridade do formato DGDS.** `detection_tables.h` lista arquivos/MD5 dos jogos; decompressão (RLE/LZW), fontes, música/MIDI e som tratados com rigor de engenharia. |

**Crédito indireto:** o projeto **xBaK** (Guido) foi a base para entender os comandos
TTM e ADS — citado tanto pelo JCOS quanto pelo jc_reborn. A seção *Johnny Castaway*
do site **Sierra Chest** (screenshots e capturas de vídeo) também ajudou a validar
comportamentos.

### Como os projetos se relacionam (fluxo de conhecimento)
```
xBaK (TTM/ADS) ─┐
                ├─► JCOS (C#, 2015) ──► castaway (JS) ──► dgds-viewer (JS/Electron)
ScummVM DGDS ───┘        │
 (formato)               └─────────► jc_reborn (C/SDL2, 2019)  ◄── engine mais fiel
```

---

## 6. Implicações para o Wilson Reborn

1. **Não partir do zero na engenharia reversa.** `jc_reborn` (gameplay) + `dgds`
   do ScummVM (formato) cobrem ~95% do necessário. As lacunas conhecidas estão
   documentadas pelos próprios autores (ver README do jc_reborn: "every scene works
   with only some inaccuracies").
2. **O conteúdo (gags, história, feriados) já está nos dados originais** — o engine
   só precisa interpretá-los corretamente. "Não perder nenhum recurso" = interpretar
   fielmente `RESOURCE.001` + replicar a lógica de escalonamento/feriados do
   `story.c`.
3. **As melhorias desejadas** (resoluções maiores, ciclo dia/noite real, etc.) são
   viáveis porque o engine é simples e os assets são vetorizáveis/escaláveis. Ver
   [07-plano-do-port-moderno](07-plano-do-port-moderno.md).
4. **Licenças:** jc_reborn e JCOS são **GPLv3**; castaway/dgds-viewer têm licença
   própria (ver `repos/*/LICENSE`); o engine DGDS do ScummVM é **GPLv2+ (ScummVM)**.
   Reutilizar código desses projetos obriga compatibilidade com GPL. Reimplementar a
   partir da *documentação* (esta base de conhecimento) evita o "contágio" de
   licença, se for desejado um licenciamento diferente.

---

### Fontes
- Site canônico de fãs: https://johnny-castaway.com/ (páginas index, common, fishing,
  swimming, reading, mermaid, pirates, seagull, visitors, leaving, annivers, story,
  unusual, bugs, johnew).
- `repos/jc_reborn/README.md` (md5/tamanhos, agradecimentos, status).
- `repos/castaway/README.md` e `repos/castaway/docs/resindex.md`.
- Wikipedia *Johnny Castaway*; Computer Gaming World (via resumo de pesquisa);
  Dynamix Wiki (Fandom); Sierra Chest.
