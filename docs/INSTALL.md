# Instalação e empacotamento

O app **`wilson`** usa os **arquivos originais** do Johnny Castaway
(`RESOURCE.MAP` + `RESOURCE.001`) — não há arte embutida. Aponte com `--data <pasta-ou-zip>`,
ou deixe os arquivos (ou o zip) no diretório de trabalho / ao lado do executável (são
auto-detectados). Sem eles, o app explica o que falta e sai.

## Onde conseguir os arquivos originais

O screensaver original (Sierra/Dynamix, 1992) está **preservado no Internet Archive**:

- Página do item: <https://archive.org/details/johnny-castaway-screensaver>
- `scrantic-run.zip` — os arquivos prontos:
  <https://archive.org/download/johnny-castaway-screensaver/scrantic-run.zip>
- `scrantic-installer.zip` — o instalador original (dados comprimidos):
  <https://archive.org/download/johnny-castaway-screensaver/scrantic-installer.zip>

**Não precisa descompactar nem instalar nada** — passe o **`.zip` direto** (qualquer um dos
dois), uma pasta já extraída, ou deixe o zip ao lado do executável:

```bash
wilson --data scrantic-run.zip         # ou: --data scrantic-installer.zip, ou --data <pasta>
```

O `wilson` aceita: uma **pasta** com os dados; o **`scrantic-run.zip`**; ou o
**`scrantic-installer.zip`** — neste último ele **descomprime** o `RESOURCE.00$`
(formato PKWARE DCL do instalador) automaticamente, em memória/temp. Auto-detecta
`scrantic-run.zip`/`scrantic-installer.zip` no diretório atual e ao lado do executável.

> **Som — automático:** os 23 efeitos originais ficam embutidos como WAVs **dentro do
> `SCRANTIC.EXE`/`.SCR`** (no instalador, dentro do `SCRANTIC.SC$` comprimido). O `wilson`
> **extrai os sons sozinho** — então qualquer um dos zips já roda **com som**, sem arquivos
> extras. (`soundN.wav` na pasta, se houver, têm prioridade.) **Copyright:** os dados são
> da Sierra/Dynamix; o Internet Archive os preserva como software histórico — use sua
> própria cópia.

## Baixar os binários

A cada **tag de versão** (`vX.Y.Z`) o workflow [`release.yml`](../.github/workflows/release.yml)
publica os artefatos numa *GitHub Release*:

- **Windows:** `wilson.scr` (o screensaver) e `wilson.exe`.
- **Linux:** `wilson-linux-x86_64.tar.gz`.

Você também pode rodar o workflow manualmente (*Actions → Release → Run workflow*) para
baixar os artefatos sem criar uma release.

## Windows (screensaver `.scr`)

Um screensaver do Windows é apenas o executável com a extensão `.scr`.

> **Primeira execução — aviso do SmartScreen.** Como o binário **não é assinado**
> ("Fornecedor desconhecido"), o Windows mostra *"O Windows protegeu o computador"*. É
> **esperado** para um app não assinado (até os artefatos do `release.yml` seriam assim) —
> não é vírus nem defeito. Para rodar: **Mais informações → Executar assim mesmo**. Para
> não ver o aviso, **desbloqueie** o arquivo antes: botão direito no `.scr`/`.exe` →
> **Propriedades** → marque **"Desbloquear"** (Unblock) → **OK**. Eliminar de vez exigiria
> **assinatura de código** (certificado pago de uma autoridade) — desnecessário para uso
> pessoal.

1. Baixe `wilson.scr`.
2. **Coloque os dados originais** (`RESOURCE.MAP` + `RESOURCE.001`) **na mesma pasta** do
   `wilson.scr` (ou aponte com `--data <dir>`). Sem eles, o screensaver não tem o que
   mostrar. *(Nota: ao instalar em `System32`, os dados precisam estar lá também — ou
   prefira rodar de uma pasta própria.)*
3. **Instalar:** clique com o botão direito em `wilson.scr` → **Instalar** (já abre a
   janela de configuração de proteção de tela com o Wilson selecionado). Ou copie o
   arquivo para `C:\Windows\System32\` e escolha **Wilson** em *Configurações → Tela de
   bloqueio → Proteção de tela*.
4. **Configurar:** o botão *Configurações* (verbo `/c`) imprime as opções atuais e o
   caminho do `config.txt` (edite-o para ajustar tela cheia, escala, som, velocidade,
   ciclo dia/noite).

> O *preview* na miniatura (verbo `/p <hwnd>`) é embutido na janelinha de pré-visualização
> (janela-filha do HWND que o Windows passa). É um recurso só de Windows; em outros
> sistemas o `/p` apenas informa e sai.

## Linux

1. Baixe e extraia: `tar -xzf wilson-linux-x86_64.tar.gz`
2. Rode com os dados originais: `./wilson --data <dir>` (ou deixe `RESOURCE.MAP`/
   `RESOURCE.001` ao lado do binário / no diretório atual). Tela cheia; qualquer
   tecla/clique encerra. Use `--windowed` para janela.
3. Para áudio, instale o ALSA em runtime se necessário (`libasound2`).

Não há um framework universal de "screensaver" no Linux; rode o `wilson` diretamente,
ou integre-o ao seu gerenciador (por exemplo, como programa externo de um daemon de
ociosidade).

## Compilar do código-fonte

No **Linux**, instale primeiro as dependências de sistema. O `wilson` linka contra o
**ALSA** em tempo de build (feature `audio`, ligada por padrão) e usa **Wayland/X11** em
runtime. Use a mesma lista do CI:

```bash
# Fedora
sudo dnf install -y alsa-lib-devel pkgconf-pkg-config wayland-devel libxkbcommon-devel libX11-devel gcc

# Debian/Ubuntu
sudo apt-get install -y libasound2-dev libwayland-dev libxkbcommon-dev libx11-dev pkg-config build-essential
```

Depois, compile:

```bash
cargo build --release -p wilson
# binário em target/release/wilson (ou wilson.exe no Windows)
```

> Sem o pacote de desenvolvimento do ALSA (`alsa-lib-devel`/`libasound2-dev`) o build
> falha em `alsa-sys` com `Package 'alsa' not found`. Se **não** quiser som, compile sem a
> feature de áudio — aí o ALSA não é necessário (o app degrada para silêncio):
>
> ```bash
> cargo build --release -p wilson --no-default-features
> # com dados embutidos: --no-default-features --features embed-data
> ```


## Build autossuficiente (dados embutidos)

Para um **único arquivo** que roda **sem** os dados ao lado (nada de `--data`), compile
com a feature `embed-data` apontando `WILSON_EMBED_DATA` para uma pasta com os dados
originais. A pasta pode ser uma cópia já extraída do `scrantic-run.zip` **ou** do
`scrantic-installer.zip` — no caso do instalador, o build **descomprime** o `RESOURCE.00$`
e tira o som do `SCRANTIC.SC$` automaticamente. O som também sai do `SCRANTIC.EXE`/`.SCR`
quando não há `soundN.wav`.

```bash
WILSON_EMBED_DATA=<pasta-extraída-do-zip-ou-instalador> cargo build --release -p wilson --features embed-data
# o binário resultante (~5 MB) embute RESOURCE.* + os 23 sons e roda de qualquer pasta
```

Os bytes são lidos **só em tempo de compilação** pela [`build.rs`](../crates/wilson/build.rs)
e nunca entram no repositório. Como esses dados são **copyright** da Sierra/Dynamix, **não**
distribua publicamente o binário com dados embutidos — esse build é para uso pessoal de quem
já tem o jogo original. (Sem `WILSON_EMBED_DATA`, a feature compila um *stub* com um aviso e
o binário não roda — útil só para o CI checar a compilação.)

## Web (WASM) — rodar no navegador

> **Jeito mais fácil — sem instalar nada:** abra a página hospedada em
> **<https://antigerme.github.io/wilson-reborn/>**, arraste seus `RESOURCE.*` (ou um `.zip`) e
> pronto (roda local, nada é enviado; fica salvo no navegador pra abrir já rodando depois).

A engine também roda **no navegador** via WebAssembly (crate [`wilson-web`](../crates/wilson-web/README.md)).
Só precisa do `wasm-bindgen-cli`; o **target wasm é adicionado automaticamente** pelo script
(via `rustup`). Há **dois modos**:

**1. Traga seus dados** (padrão) — **arraste** seus `RESOURCE.MAP`/`RESOURCE.001` (+ `SCRANTIC.EXE`
p/ som) **ou um `scrantic-run.zip` / `scrantic-installer.zip`** (lidos localmente, nada é enviado).
Os dados ficam **salvos no navegador** (IndexedDB) e o screensaver **inicia sozinho** na próxima
visita; use **🗑 Forget** pra limpar e escolher outros dados.
É o modo seguro pra hospedar:

```bash
cargo install wasm-bindgen-cli          # a versão precisa casar com o crate wasm-bindgen
./crates/wilson-web/build-web.sh        # auto-adiciona o target wasm + gera crates/wilson-web/web/
python3 -m http.server -d crates/wilson-web/web 8000
# abra http://localhost:8000/ e arraste os arquivos OU o .zip (run ou instalador)
```

**Opções na URL** (paridade com o desktop) — `?fullscreen` (tela cheia + fundo preto),
`?scale=fit|stretch|integer` e `?filter=linear|nearest` (**padrão `fit`+`linear`, igual ao
desktop**), `?speed=25–400`, `?day=1–11`, `?dissolve`, `?story[&story_secs=N]`, `?daynight=real`,
`?intro=0`, `?intro_secs=1–30` (hold do intro, padrão 3 s), `?mute`, `?volume=0–100`, `?seed=N`.
Há botões **🔊/🔇 + volume** e **⛶ tela cheia** (que mantém a tela acordada via **Wake Lock**); a
UI/cursor somem após alguns segundos parados. (Com `?dissolve`, o dissolve também cobre o intro→1ª cena.)

**2. Autossuficiente** (uso pessoal) — embute os `RESOURCE.*` **+ os sons** (do `SCRANTIC.EXE`)
no `.wasm` (feature `embed-data`), então a página **abre e roda** sem seletor. Aponte
`WILSON_EMBED_DATA` (ou use o empacotador):

```bash
WILSON_EMBED_DATA=<pasta-com-RESOURCE.*> ./crates/wilson-web/build-web.sh   # direto
scripts/build-embedded.sh --web <data-dir>                                 # ou pelo empacotador
```

Pelo empacotador, **`--web`** coloca o bundle em `<out>/web/` — **com** `<data-dir>` ele é
autossuficiente (dados embutidos); **sem** `<data-dir>` é o modo "traga seus dados":

```bash
scripts/build-embedded.sh --web                 # só o web, traga-seus-dados
scripts/build-embedded.sh --web <data-dir>      # desktop (embutido) + web (embutido)
```

> O bundle **autossuficiente** contém o jogo (copyright) — **uso pessoal, não hospede/redistribua.**
> Os artefatos gerados (`wilson_web.js`/`_bg.wasm`) são git-ignored; só o `index.html` é
> versionado. **Som ligado por padrão** (Web Audio; botão 🔊/🔇 pra mutar) — começa no primeiro
> clique (política de autoplay dos navegadores). No modo "traga seus dados", o som precisa do
> `SCRANTIC.EXE` (onde os efeitos ficam embutidos); no autossuficiente já vem assado.

## Ícone (Windows e macOS)

Os binários Windows (`wilson.exe`/`wilson.scr`) já vêm com um **ícone próprio do Wilson
Reborn** — arte original (ilha + palmeira) em `crates/wilson/assets/wilson.ico` (gerada por
`assets/make_icon.py`), embutida pela [`build.rs`](../crates/wilson/build.rs). É um ícone
**nosso**, distribuível sem problema.

No build pessoal **`embed-data`**, em vez do nosso, é usado o **ícone original** do Johnny
Castaway, extraído do **seu** `SCRANTIC.EXE`/`.SCR` (ou de um `SCRANTIC.ICO` na pasta de
dados) em tempo de compilação — é **copyright**, então **nunca** é commitado nem entra nos
binários públicos das releases.

> Na cross-compilação Linux→Windows (`scripts/build-embedded.sh`) o ícone precisa do
> `windres` do mingw (`x86_64-w64-mingw32-windres`, do binutils do mingw). Sem ele o build
> segue **sem** o ícone (só um aviso, não falha). No build nativo Windows (MSVC) é automático.

No **macOS**, o `.saver` (`crates/wilson-saver/macos/build-saver.sh`) inclui o **mesmo ícone
nosso** como `wilson.icns` no bundle (`Contents/Resources/`, referenciado por
`CFBundleIconFile` no `Info.plist`), gerado pelo mesmo `make_icon.py`. (O ícone original de
1992 é 32×32/16 cores — fica ruim nos tamanhos do macOS, então no macOS usamos sempre o nosso.)

### Gerar embedded para várias plataformas (a partir do Linux)

O script [`scripts/build-embedded.sh`](../scripts/build-embedded.sh) gera, de uma vez, os
binários **embedded** do seu Linux:

```bash
scripts/build-embedded.sh <pasta-dos-dados> [pasta-de-saida]
```

Ele começa por um **diagnóstico (preflight)** que mostra `[ok]`/`[--]` para cada
pré-requisito (dados, `cargo`, `rustup`, ALSA, alvo Windows, linker mingw) com o comando
exato pra corrigir o que faltar. Use `--check` para **só** ver o diagnóstico, sem compilar:

```bash
scripts/build-embedded.sh --check <pasta-dos-dados>
```

> **⚠️ `--fetch-ia` — baixar os originais do Internet Archive (opt-in, uso pessoal).** Em vez
> de passar `<pasta-dos-dados>`, você pode usar **`--fetch-ia`** pra o script **baixar os dados
> originais** do Internet Archive (`scrantic-run.zip`, verificado por **SHA-256** fixo),
> descompactar num diretório temporário e embutir. **São dados COPYRIGHT** (Sierra/Dynamix →
> Activision/Microsoft): o binário gerado **contém o jogo** e é **só para uso pessoal — não
> redistribua**. O script imprime um **aviso legal explícito** (EN+PT) e exige você digitar
> `I ACCEPT` (ou passar `--i-accept-legal-responsibility` pra rodar sem interação). É
> **bloqueado em CI** de propósito. Use **somente** se você tem direito a uma cópia — **toda a
> responsabilidade legal é sua**.
>
> ```bash
> scripts/build-embedded.sh --fetch-ia            # baixa (após aceitar o aviso) e compila
> scripts/build-embedded.sh --check --fetch-ia    # só diagnóstico — não baixa nada
> ```

Alvos:

- **Linux** `x86_64` (nativo) → `wilson-linux-x86_64`. Precisa das deps de build do Linux
  (ALSA etc. — ver "Compilar do código-fonte" acima).
- **Windows** `x86_64` (cross via mingw-w64) → `wilson.exe` + `wilson.scr` (runtime mingw
  estático, então é um arquivo só). Precisa do **rustup** (para adicionar o alvo), do alvo
  e do mingw:
  ```bash
  # rustup (o cargo do dnf NÃO traz rustup, necessário p/ adicionar alvos):
  sudo dnf install -y rustup && rustup-init -y && source "$HOME/.cargo/env"
  rustup target add x86_64-pc-windows-gnu
  sudo dnf install -y mingw64-gcc        # Fedora  (Debian/Ubuntu: gcc-mingw-w64-x86-64)
  ```
  O alvo Windows usa o áudio nativo do Windows (WASAPI) — **não** precisa de ALSA.
- **macOS**: não dá para gerar a partir do Linux (precisa do SDK da Apple/osxcross). Faça
  **num Mac**: `WILSON_EMBED_DATA=<dir> cargo build --release -p wilson --features embed-data`
  e, para o screensaver, `crates/wilson-saver/macos/build-saver.sh`.

Se um alvo estiver sem pré-requisitos, o script **pula** aquele alvo (com a dica de
correção) em vez de falhar no meio. Saída em `target/embedded/` por padrão. Os binários
contêm os dados copyright — **uso pessoal**.

## Publicar uma release (mantenedor)

```bash
git tag v0.1.0
git push origin v0.1.0   # dispara o workflow release.yml e cria a Release com os artefatos
```
