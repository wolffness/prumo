# Handoff — Prumo: PT-BR, distribuição e módulo advisor (2026-07-18)

## Propósito
Sessão longa que entregou: distribuição via Homebrew, rename dinâmico
tuxedo→prumo, interface e parser NL em PT-BR, autocomplete na captura rápida,
hotkey para abrir o app, e o **incremento 1 do módulo advisor de IA** (opt-in).
Este handoff permite retomar do ponto exato.

## Estado atual (tudo em `main`, CI verde)
- Repo: `wolffness/prumo` (upstream `webstonehq/tuxedo`). Branch `main`.
- Último trabalho: `feat: opt-in AI advisor module (increment 1)` — CI success.
- Release Homebrew ativa: `v2026.7.1-prumo2`. Tap `wolffness/homebrew-prumo`
  (formula pina por `tag` + `revision`, instala só o binário `prumo`).
- Fronteira do rename: Cargo gera 2 binários (`tuxedo`, `prumo`) do mesmo
  `src/main.rs`; todo texto visível segue o nome invocado via `src/brand.rs`
  (`app_name()`, `is_prumo()`, `tr(en, pt)`). Config/cache paths e env vars
  seguem `tuxedo` (merges baratos com upstream). Fallback "tuxedo"/inglês nos
  testes → snapshots do upstream intactos.

## O que já foi entregue nesta sessão
1. **Homebrew**: `brew install wolffness/prumo/prumo`. Job `update-tap` do
   `release.yml` (apontava p/ tap do upstream) foi **removido**.
2. **Rename dinâmico** + **UI em PT-BR** via `brand::tr` (ajuda, status bar,
   painéis FILTROS/PROJETOS/CONTEXTOS/DETALHES, grupos ATRASADAS/HOJE/…,
   configurações, welcome/empty, note panel, flashes).
3. **README** 100% PT-BR; badges → CI/release do fork.
4. **Parser NL PT** (`src/nl.rs`): prosa de prioridade (`prioridade alta/média/
   baixa/a-c` + invertido), projeto/contexto (`projeto casa`, `contexto banco`),
   antecedência (`mostrar 3 dias antes [do vencimento]`), e recorrência mensal
   por palavra (`todo dia primeiro`/`último` via `RecHint::MonthLastDay`).
5. **Captura rápida** (agente macOS): autocomplete de `+proj`/`@ctx` (Enter
   aceita sugestão aberta); **`⌥[`** abre/levanta o app (Carbon hotkey por id,
   `CapturePanel.swift`); `⌥]` segue a captura.
6. **Advisor incremento 1** (ver abaixo).

## Módulo advisor — desenho e estado
Arquitetura decidida com o usuário:
- **Opt-in, desligado por padrão**, desacoplado — núcleo funciona sem IA.
- Backend escolhível: **Ollama** (local, padrão) ou **Claude** (o usuário usa
  Claude). Chave via **`ANTHROPIC_API_KEY`** (env), nunca no config.toml.
- Só-leitura: imprime sugestão, nunca escreve no todo.txt.
- **GitHub**: reusar o `gh` já logado (decisão do usuário) — detectar+orientar
  em runtime, sem OAuth próprio. Ainda NÃO implementado (é o incremento 2).

Incremento 1 entregue (`src/advisor/mod.rs`, `src/cmd/mod.rs`, `src/config.rs`,
`src/lib.rs`):
- Comando `prumo advisor prioritize` (aliases `pri`/`priorizar`).
- Config: `advisor = on/off`, `advisor_backend = ollama|claude`,
  `advisor_model = <modelo>` (default por backend; Claude → `claude-opus-4-8`).
- Backends via `curl` (Rust não tem SDK oficial): Ollama
  `localhost:11434/api/generate`; Claude `api.anthropic.com/v1/messages`
  (`x-api-key`, `anthropic-version: 2023-06-01`, `max_tokens: 1024`).
- Helpers JSON manuais (sem serde) com testes.
- 6 testes unitários passando.

## Blocking questions (decidir antes do incremento 2)
1. Confirmar que o incremento 2 (GitHub) usa `gh-axi`/`gh` já logado, puxando
   issues/PRs abertos e priorizando junto do todo.txt. Links do usuário:
   github.com/kunchenguid/axi e github.com/kunchenguid/gh-axi.
2. Ordem das outras features aprovadas (todas independentes do advisor):
   "ver só N", token `est:`, ressurgir esquecidas, modo foco, weekly review.

## Tarefas preparatórias (copy-paste)
```bash
cd ~/Documents/Projetos/Prumo
git log --oneline -8
# testar advisor com Claude:
#   editar ~/.config/tuxedo/config.toml -> advisor = on / advisor_backend = claude
#   export ANTHROPIC_API_KEY=...
#   prumo advisor prioritize
gh-axi run list --repo wolffness/prumo --limit 3
```

## Pendência menor (decisão do usuário)
`/opt/homebrew/bin/tuxedo` avulso (2026.6.3, fora do brew, desatualizado). Não
atrapalha o `prumo`. Remover com `rm /opt/homebrew/bin/tuxedo` se quiser.

## Entry prompt (colar como primeira mensagem na próxima sessão)
```
Retomando o Prumo (fork wolffness/prumo). Leia
docs/handoffs/2026-07-18-advisor-e-ptbr.md e a memória do projeto. Contexto:
já entreguei Homebrew, rename dinâmico tuxedo→prumo, UI+README+parser NL em
PT-BR, autocomplete/hotkey na captura macOS, e o incremento 1 do módulo advisor
(prumo advisor prioritize, opt-in, Ollama/Claude, só-leitura). Tudo em main, CI
verde. Próximo: incremento 2 do advisor = integração GitHub reusando o gh já
logado (puxar issues/PRs abertos e priorizar junto do todo.txt). Também há
5 features aprovadas independentes do advisor: "ver só N", token est:,
ressurgir esquecidas, modo foco, weekly review. Me pergunte por onde seguir.
```
