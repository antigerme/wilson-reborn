# Instalação e empacotamento

O app **`wilson`** é um único binário **standalone** — traz o asset pack recriado
embutido, então não precisa de arquivos externos. (Para usar os dados originais, passe
`--data <dir>` com os seus `RESOURCE.MAP`/`RESOURCE.001`.)

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
2. **Instalar:** clique com o botão direito em `wilson.scr` → **Instalar** (já abre a
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
2. Rode: `./wilson` (tela cheia; qualquer tecla/clique encerra). Use `--windowed` para
   rodar em janela.
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
