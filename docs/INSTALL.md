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

1. Baixe `wilson.scr`.
2. **Coloque os dados originais** (`RESOURCE.MAP` + `RESOURCE.001`) **na mesma pasta** do
   `wilson.scr` (ou aponte com `--data <dir>`). Sem eles, o screensaver não tem o que
   mostrar. *(Nota: ao instalar em `System32`, os dados precisam estar lá também — ou
   prefira rodar de uma pasta própria.)*
3. **Instalar:** clique com o botão direito em `wilson.scr` → **Instalar** (já abre a
   janela de configuração de proteção de tela com o Wilson selecionado). Ou copie o
   arquivo para `C:\Windows\System32\` e escolha **Wilson** em *Configurações → Tela de
   bloqueio → Proteção de tela*.
3. **Configurar:** o botão *Configurações* (verbo `/c`) imprime as opções atuais e o
   caminho do `config.txt` (edite-o para ajustar tela cheia, escala, som, velocidade,
   ciclo dia/noite).

> O *preview* na miniatura (verbo `/p`) ainda não é embutido — a miniatura fica em
> branco, mas o screensaver funciona normalmente em tela cheia.

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

```bash
cargo build --release -p wilson
# binário em target/release/wilson (ou wilson.exe no Windows)
```

## Publicar uma release (mantenedor)

```bash
git tag v0.1.0
git push origin v0.1.0   # dispara o workflow release.yml e cria a Release com os artefatos
```
