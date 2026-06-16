# Instalação e empacotamento

O app **`wilson`** usa os **arquivos originais** do Johnny Castaway
(`RESOURCE.MAP` + `RESOURCE.001`) — não há arte embutida. Coloque esses arquivos **no
mesmo diretório do executável** (ou no diretório de trabalho), ou aponte com
`--data <dir>`. Sem eles, o app explica o que falta e sai.

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
originais (`RESOURCE.MAP` + `RESOURCE.001` e, se quiser som, os `soundN.wav`):

```bash
WILSON_EMBED_DATA=<dir-com-os-dados> cargo build --release -p wilson --features embed-data
# o binário resultante (~5 MB) embute RESOURCE.* + soundN.wav e roda de qualquer pasta
```

Os bytes são lidos **só em tempo de compilação** pela [`build.rs`](../crates/wilson/build.rs)
e nunca entram no repositório. Como esses dados são **copyright** da Sierra/Dynamix, **não**
distribua publicamente o binário com dados embutidos — esse build é para uso pessoal de quem
já tem o jogo original. (Sem `WILSON_EMBED_DATA`, a feature compila um *stub* com um aviso e
o binário não roda — útil só para o CI checar a compilação.)

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
