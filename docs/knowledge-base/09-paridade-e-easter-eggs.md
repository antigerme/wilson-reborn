# 09 — Auditoria de paridade e easter eggs

> Resposta direta a "**não perder nenhum recurso**": confronta a [bíblia de
> conteúdo](02-biblia-de-conteudo.md) (tudo que o original tem) com o que o Wilson
> Reborn já faz. Atualizar quando recursos forem (re)implementados.

## Conclusão em 30 segundos

O Wilson Reborn tem **dois caminhos de conteúdo**, e a paridade é diferente em cada um:

1. **Com `--data` (dados originais do usuário): paridade TOTAL de conteúdo.** O engine
   **interpreta os scripts originais** (`.ADS`/`.TTM` do `RESOURCE.001`), então **todos
   os 63 cenas, gags, easter eggs, visitantes e beats de enredo aparecem exatamente como
   no original** — não reimplementamos cada gag, nós **executamos os mesmos bytecodes**.
   Validado de ponta a ponta (ver [08](08-decisoes-e-status.md), teste `real_data`).
2. **Pack recriado embutido (copyright-free): lógica completa, visual ainda placeholder.**
   O **diretor** sorteia corretamente as 63 cenas, os 11 dias, os beats de enredo, os
   feriados, maré/noite/jangada — mas o **TTM recriado** desenha o Johnny **parado**
   (mais o caminhar entre spots). Ou seja, a *coreografia* dos gags ainda não foi
   redesenhada em arte própria. **Recriar as 63 animações é trabalho de conteúdo futuro.**

> Em resumo: **nada do original se perde** — está tudo acessível via `--data`. O que
> falta é **arte recriada** para cada gag, para que o pack standalone seja tão rico
> quanto o original sem precisar dos dados do usuário.

## Lógica do diretor — paridade ✅ (com testes)

Tudo isto é portado fielmente de `story.c`/`story_data.h` e **coberto por testes**
(`crates/wilson-engine/src/story.rs`):

| Recurso | Estado | Onde |
|---|---|---|
| Tabela de **63 cenas** (10 `.ADS`) | ✅ | `STORY_SCENES` (teste `table_has_63_scenes`) |
| Arco de **11 dias** + avanço por data real + reinício | ✅ | `Director::advance_day` (teste `advance_day_clamps_and_wraps`) |
| **Beats de enredo** dos 11 dias (Mary/Suzy/Johnny) | ✅ | campo `day` (teste `day_beats_match_the_story`) |
| **4 feriados** com faixas de data exatas | ✅ | `holiday_for_date` (teste `holidays`) |
| **Jangada** (5 estágios por dia) | ✅ | `raft_for_day` (teste `night_and_raft`) |
| **Maré** baixa/alta + **noite** | ✅ | `island_from_scene`, `is_night` |
| **Dia/noite 24h** (melhoria opcional) | ✅ | `DayNight` (teste `night_24h_cycle`) |
| **Pathfinding** 2ª ordem + **walk** entre spots | ✅ | `path`/`walk` (testes próprios) |
| **Props de feriado** desenhados na ilha | ✅ | `island.rs` (compostos no cenário) |

### Feriados (faixas confirmadas iguais ao original)

| Feriado | Faixa | `Holiday` |
|---|---|---|
| Ano Novo | 29/12 → 01/01 | `NewYear` |
| São Patrício | 15/03 → 17/03 | `StPatrick` |
| Halloween | 29/10 → 31/10 | `Halloween` |
| Natal | 23/12 → 25/12 | `Christmas` |

A bíblia nota o desejo de **tabela extensível** (ex.: 4 de Julho) — possível melhoria
futura (precisa de novos `Holiday` + props; degrada com `--data`, cujo `HOLIDAY.BMP` só
tem 4 sprites).

## Gags, personagens e easter eggs — status

Legenda: **D** = aparece com `--data` (script original) · **R** = arte recriada no pack
embutido.

| Recurso (bíblia §3–§10) | D | R | Observação |
|---|:--:|:--:|---|
| Pesca (capturas comuns/raras, polvão, tubarão-esqui, ambidestria) | ✅ | ❌ | script original roda; arte recriada pendente |
| Natação/mergulho + júri de bichos | ✅ | ❌ | |
| Banho + gaivota ladra + susto do tubarão | ✅ | ❌ | |
| Leitura (livro de cabeça pra baixo, cochilo→coco) | ✅ | ❌ | |
| Dormir/roncar + amarração pelos piratas | ✅ | ❌ | |
| Fogo/cozinhar (polvo no rosto) | ✅ | ❌ | |
| Cocos (quiques, quebrar na árvore) | ✅ | ❌ | |
| **Jangada** + **SOS na garrafa** (mini-Johnny, dia 2) | ✅ | ⚠️ | jangada cresce no pack recriado; o gag da garrafa não |
| Castelo de areia → piratas King Kong | ✅ | ❌ | |
| Cooper / telescópio | ✅ | ❌ | |
| Dança da chuva (gota → raio) | ✅ | ❌ | |
| **Mary, a sereia** (6 interações + beats) | ✅ | ❌ | diretor escolhe os dias; visual pendente |
| **Suzy** (resort/beijo/puxão de orelha) | ✅ | ❌ | |
| Gaivota (5 gags) | ✅ | ❌ | |
| Piratas (King Kong + Gulliver, ovo no peito) | ✅ | ❌ | |
| Visitantes (lancha, biplano, helicóptero, terminator, navio gigante, x3 pelados…) | ✅ | ❌ | `VISITOR.ADS` roda com `--data` |
| Easter eggs raros (Johnny fantasma, bolas de prata, relógio real, derreter, "feeding the fishes", "THE END/Home Again") | ✅ | ❌ | |
| **Feriados** (props na ilha) | ✅ | ✅ | abóbora/pote/pinheiro/fogos recriados |
| **Som** (`sound0..24`, `sound0` nos beats) | ✅ | ➖ | toca os `.wav` originais com `--data`; o pack não traz `.wav` (copyright) |

## Bugs "charme" como easter egg opcional (futuro)

A bíblia §12 lista bugs do original; alguns viraram **piadas queridas** ("ilha gigante",
"dezenas de Johnnys", "gêmeos"). Ideia de melhoria: um **modo easter-egg opcional** que
os reproduz de propósito. Não implementado (não é regressão — são bugs, não recursos).

## Próximos passos para fechar a paridade *visual* do pack recriado

Em ordem de impacto (cada um é um incremento de conteúdo):
1. **Animações recriadas por categoria** (pesca, banho, leitura, dormir, cocos…), para o
   `STAND/ACTIVITY/FISHING/...` mostrarem ações distintas em vez do Johnny parado.
2. **Personagens recriados** (Mary, Suzy) para os beats dos dias 1/3/4/5/7/8/9.
3. **Visitantes recriados** (`VISITOR.ADS`) e **easter eggs raros**.
4. **SOS na garrafa** (dia 2) e **dança da chuva**.

Enquanto isso, **`--data` entrega 100% do conteúdo original**.
