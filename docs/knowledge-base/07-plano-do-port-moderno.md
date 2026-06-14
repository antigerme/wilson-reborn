# 07 — Plano do Port Moderno (Wilson Reborn)

> Síntese voltada à ação: como construir um clone **moderno, extremamente portável
> (Windows/Linux), em resoluções melhores e com melhorias**, **sem perder nenhum
> recurso** do original. Recomendações + roadmap. Decisões marcadas com 🟦 são pontos a
> confirmar com você.

---

## 1. Objetivos (do projeto)

1. **Portabilidade extrema** Windows/Linux (e, de brinde, web/macOS quando possível).
2. **Linguagem moderna**.
3. **Resoluções melhores** que os 640×480 fixos / 16 cores do original.
4. **Melhorias** além do original (sem quebrá-lo).
5. **Paridade total** com o original — todos os eventos, gags, history, easter eggs,
   feriados (ver [checklist na bíblia §14](02-biblia-de-conteudo.md#14-checklist-de-paridade-resumo-não-perder-nada)).

---

## 2. Stack recomendada 🟦

**Recomendação primária: Rust** + um *pixel buffer* apresentado por GPU (`pixels`/`wgpu`),
espelhando o modelo de **blitter em camadas** do original.

Por quê: binário único estático, **cross-compile trivial** (Win/Linux/macOS), **roda em
WebAssembly** (versão web "de graça"), seguro em memória, ótimo para um processo
long-running de baixo consumo. O engine original é um compositor de surfaces 2D — um
*framebuffer* acelerado (`pixels` sobre `wgpu`) preserva 1:1 a lógica de composição
(fundo → zonas → threads → feriado) e ainda dá escalonamento/HiDPI.

**Alternativas fortes** (todas atingem os objetivos):

| Stack | Prós | Contras |
|---|---|---|
| **Rust + `pixels`/`wgpu`** (rec.) | binário único, WASM, perf, seguro | curva de aprendizado |
| **Rust + `macroquad`** | API simples, web nativa, simples de empacotar | menos controle fino |
| **Go + Ebitengine** | builds e cross-compile simplíssimos, WASM, 2D pronto | binários maiores; GC |
| **TypeScript + Canvas + Tauri** | reusa `castaway`/`dgds-viewer`; web-first; Tauri ≪ Electron | desktop depende de WebView |
| **C + SDL2** (= jc_reborn) | reusa quase tudo do jc_reborn | C "menos moderno"; GPLv3; sem web fácil |

> Se o objetivo nº1 for **menor esforço reaproveitando código pronto**, o caminho é
> **TypeScript** (parsers de `dgds-viewer` + metadados de `castaway`) ou **C/SDL2**
> (fork conceitual do jc_reborn). Se for **melhor produto de longo prazo**, **Rust**.

---

## 3. Arquitetura proposta

Espelhar as 4 camadas limpas do jc_reborn ([05 §10](05-arquitetura-do-engine.md)):

```
┌─────────────────────────────────────────────────────────────┐
│ Plataforma: screensaver Win (.scr) │ Linux (XScreenSaver/    │
│ standalone/Wayland) │ app standalone │ web (WASM) │ wallpaper │
├─────────────────────────────────────────────────────────────┤
│ Backend de Render/Áudio (trait/interface): pixels|wgpu|canvas │
├─────────────────────────────────────────────────────────────┤
│ Lógica de jogo: diretor (story), walk, ilha, dia/feriado      │
├─────────────────────────────────────────────────────────────┤
│ VMs de bytecode: interpretador TTM + ADS                      │
├─────────────────────────────────────────────────────────────┤
│ I/O de dados: parser RESOURCE.MAP/.001 + RLE/LZW + tipos      │
└─────────────────────────────────────────────────────────────┘
```
**Backend abstrato** (uma interface `Renderer`/`Audio`) permite o mesmo core rodar em
desktop, web e como screensaver — chave para "portabilidade extrema".

---

## 4. Independência de resolução (o "rodar melhor")

O original é **640×480, 16 cores, sprites bitmap**. Estratégias (combináveis):

1. **Escalonamento inteiro nearest-neighbor** (pixel-perfect): renderiza no buffer 640×480
   e escala ×2/×3/×N para a tela — fiel, nítido, trivial. *MVP.*
2. **Escalonamento para qualquer resolução / HiDPI** com opção de filtro (nearest vs
   suave) e *letterboxing* para manter proporção.
3. **Reposicionamento em telas grandes:** o engine já desenha a ilha em posição aleatória
   (`VARPOS_OK`); ampliar as faixas para telas widescreen/4K dá mais "espaço de mar".
4. **Pacote de assets HD (futuro):** como os sprites são pequenos e estilizados, dá para
   **re-desenhar em alta resolução** (ou vetorizar) um "HD asset pack" opcional, mantendo
   o pixel-art original como default.
5. **Multi-monitor** e **resolução nativa** detectada automaticamente.

---

## 5. Assets e estratégia legal 🟦

Os dados originais (`RESOURCE.*`, sprites, sons) são **copyright Sierra/Dynamix**
([01 §nota legal](01-historia-e-creditos.md)). Opções:

- **(A) BYO data (padrão dos clones):** o Wilson Reborn é o *engine* livre; o usuário
  fornece seus `RESOURCE.MAP`/`RESOURCE.001` (que possui). Simples e legalmente seguro.
- **(B) Asset pack recriado:** arte/sons **novos** feitos do zero, redistribuíveis →
  versão 100% standalone e legal. Mais trabalho, mas é o caminho para distribuir
  "completo".
- **(C) Híbrido:** engine + loader que aceita **tanto** os dados originais **quanto** um
  asset pack recriado (formato próprio, ex.: JSON + PNG/sprites + ogg). Recomendado:
  abre as duas portas.

> Os 3 conjuntos que **não** vêm do `RESOURCE.001` (`story_data.h`, `walk_data.h`,
> `calcpath_data.h`) precisam ser portados/recriados de qualquer forma
> ([03 §8](03-dados-originais-e-formatos.md#8-dados-que-não-estão-no-resource001)).

---

## 6. Empacotamento por plataforma

- **Windows:** um `.scr` é apenas um `.exe` que responde a `/s` (show), `/p` (preview),
  `/c` (config). O core compila para `.exe` e expõe esses argumentos.
- **Linux:** standalone fullscreen (modo screensaver clássico = "qualquer tecla sai"); e/ou
  integração com **XScreenSaver** (modo janela via `-window-id`) e Wayland (ext-idle).
- **App standalone / "live wallpaper":** mesmo binário, modo janela/desktop.
- **Web (WASM):** demo em navegador (como o castaway/dgds-viewer já fazem).
- **macOS (bônus):** `.saver` bundle se desejado.

---

## 7. Roadmap de melhorias (sem quebrar o original) 🟦

Combinando a roadmap do `castaway` + oportunidades desta pesquisa. Tudo **opcional/
configurável**, com um "modo clássico" 100% fiel como default.

**Visual/tempo**
- Ciclo **dia/noite de 24h** real (em vez de 8h), opcionalmente baseado em **geolocalização**
  (nascer/pôr do sol reais).
- **Marés reais** por localização; nuvens em **movimento**; ondas/parallax extras.
- **Resoluções HD**, multi-monitor, HiDPI, asset pack HD opcional.

**Conteúdo**
- **Feriados extensíveis/configuráveis** (a tabela é pequena —
  [bíblia §9](02-biblia-de-conteudo.md#9-datas-comemorativas--feriados-annivers--lógica-de-storyc));
  investigar o **4 de Julho** citado pela Wikipedia e adicionar datas regionais (ex.:
  feriados brasileiros).
- **Modo "bugs clássicos"** como easter egg (ilha gigante, dezenas de Johnnys —
  [bíblia §12](02-biblia-de-conteudo.md#12-bugs-originais-catalogados-em-bugs)).
- Tocar a **história completa em sequência** (não só por dia real) — modo "story".

**Qualidade de vida**
- **UI de configuração** (velocidade, som, ciclo, feriados, escala/filtro, monitores).
- **Estatísticas** (horas tocadas, atividades vistas) — ideia do castaway.
- **Acelerar o tempo** / pular para um dia da história (ótimo para testes e para o usuário
  ver tudo).

---

## 8. Plano faseado

| Fase | Entregável | Foco |
|---|---|---|
| **0 — Fundação** | Parser de `RESOURCE.MAP/.001` + RLE/LZW; dump de recursos (validar contra `jc_reborn dump`) | [03](03-dados-originais-e-formatos.md) |
| **1 — VMs** | Interpretadores TTM + ADS; tocar **uma cena** isolada | [04](04-engine-scripting-opcodes.md) |
| **2 — Render** | Backend de pixel-buffer em camadas + paleta + sprites + som; tocar cena com áudio | [05](05-arquitetura-do-engine.md) §7–8 |
| **3 — Ilha & walk** | Fundo/maré/noite/nuvens/jangada; spots A–F + pathfinding + animação de caminhada | [05](05-arquitetura-do-engine.md) §5–6 |
| **4 — Diretor** | `storyPlay`: ciclo de 11 dias, seleção de cenas, feriados → **paridade** | [02](02-biblia-de-conteudo.md), [05](05-arquitetura-do-engine.md) §4 |
| **5 — Empacotar** | `.scr` Windows + standalone Linux + (web) | §6 |
| **6 — Melhorias** | Resoluções HD, dia/noite 24h, config UI, etc. | §7 |

**MVP fiel = fim da Fase 5.** Validar paridade pela
[checklist da bíblia §14](02-biblia-de-conteudo.md#14-checklist-de-paridade-resumo-não-perder-nada).

---

## 9. Questões em aberto / riscos

| Item | Detalhe | Encaminhamento |
|---|---|---|
| **11 vs ~120 dias** | Engines usam ciclo de **11 dias**; Wikipedia diz ~120 | Seguir os dados (11), deixar configurável; investigar `scrantic.ini`/`NumDays` |
| **Independence Day** | Citado pela Wikipedia, ausente do site/jc_reborn | Procurar arte/cena nos dados; tabela de feriados extensível |
| **RLE2 (método 3)** | Só o JCOS menciona; não usado(?) | Implementar só se aparecer nos dados |
| **VQT:** | Imagens VQ não decodificadas pelo ScummVM | Verificar se o JC usa; senão, ignorar |
| **Aproximações do jc_reborn** | walk/escalonador/posição da ilha são observacionais | Refinar via **disassembly do `SCRANTIC.SCR`** se quiser 100% |
| **Divergências de opcode** | DELAY ×10/×20, `0xA100` rect vs window | Validar rodando contra os dados ([04 §4](04-engine-scripting-opcodes.md#4-divergências-entre-implementações-atenção-ao-portar)) |
| **Dados não-recurso** | `walk/story/calcpath` vêm do exe/observação | Portar verbatim ou re-extrair do `SCRANTIC.SCR` (offset 0x188ea) |

---

## 10. Decisões para confirmar 🟦
1. **Linguagem/stack** (Rust recomendado; alternativas em §2).
2. **Estratégia de assets** (BYO data / asset pack recriado / híbrido — §5).
3. **Escopo do MVP** (paridade fiel primeiro vs. já incluir melhorias).
4. **Licença** do Wilson Reborn (afeta o quanto reusar dos projetos GPL — [06 §4](06-projetos-de-referencia.md#4-matriz-de-licenças-resumo)).
