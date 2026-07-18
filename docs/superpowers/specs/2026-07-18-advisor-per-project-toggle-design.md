# Design — Advisor liga/desliga por projeto + indicador visual

Data: 2026-07-18
Status: aprovado

## Propósito

Permitir ligar/desligar o advisor de IA **por projeto** (não global), via
comando, com um símbolo no painel PROJETOS indicando quais estão ativos.
Também documentar no `?` os comandos de liga/desliga e o acesso ao SHELL.

## Decisões

- Símbolo de IA ativa: **`✦`** (presença/ausência, não cor — daltonismo).
- O `advisor = on` global legado é **ignorado**: o controle é 100% por projeto
  (opt-in por projeto; padrão nada ligado). `advisor_backend`/`advisor_model`
  continuam para escolher o backend.

## Componentes

### 1. Config — `advisor_projects`

Nova `advisor_projects: Vec<String>` no `Config`. Serializado uma linha por
projeto ligado: `advisor_project.<nome> = on`. Parse: adiciona ao set se o
valor for verdadeiro (`on`/`true`/`1`); ignora caso contrário.

### 2. Comandos (cmd/mod.rs)

- `advisor on +<projeto>` → adiciona ao set, grava config.
- `advisor off +<projeto>` → remove do set, grava config.
- `advisor prioritize +<projeto>` → só roda se o projeto estiver ligado; senão
  avisa: "advisor desligado para +X — ligue com `advisor on +X`".
- `advisor prioritize` (sem projeto) → prioriza todos os projetos ligados; se
  nenhum, avisa para ligar um.
- Funciona pelo mini-terminal (`! prumo advisor on +X`).

`AdvisorConfig` deixa de ter o campo `enabled` (backend + model só); o gating
passa a ser por projeto na camada cmd.

### 3. Indicador no painel PROJETOS (ui/filters.rs)

Projetos ligados recebem `✦` numa coluna-marcador à esquerda da linha;
desligados mostram espaço. App passa a guardar `advisor_projects` (populado em
`new_with_done` e `reload_config`, espelhando `saved_filters`), com
`advisor_project_enabled(name) -> bool`.

### 4. Tela de ajuda `?` (ui/help.rs)

Acrescenta:
- `advisor on/off +projeto` — liga/desliga o advisor por projeto.
- `/` + `! comando` — roda comando de shell (mini-terminal).

## Testes

- Config: round-trip de `advisor_projects`; parse de `= on`/`= off`.
- cmd: `advisor on/off` altera o set; prioritize recusa projeto desligado,
  aceita ligado; sem projeto ligado avisa.
- UI: helper puro do símbolo do projeto (ligado→`✦`, desligado→vazio).

## Fora de escopo (YAGNI)

- Tecla no TUI para alternar (fica para incremento seguinte).
- Migração do `advisor = on` legado (ignorado de propósito).
