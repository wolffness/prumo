# Design — Visão de Issues do GitHub no TUI + priorização por objetivo

Data: 2026-07-18
Status: aprovado (fatiado A→B)

## Propósito

Trazer as issues do repo vinculado a um projeto para **dentro do Prumo**, numa
visão dedicada, e deixar o advisor **ranquear a importância** delas em direção a
um **objetivo** definido pelo usuário. Fecha o ciclo planejamento→execução:
ver → priorizar por meta → importar o que atacar para o todo.txt.

## Restrições herdadas

- Advisor opt-in e **read-only no GitHub** (nunca escreve lá). Importar mexe só
  no todo.txt local.
- Reusa o `gh` logado (sem OAuth) e o backend LLM configurado (Claude/Ollama).
- Acessibilidade: níveis por **símbolo**, nunca cor (daltonismo).

## Fatiamento

- **Fatia A** — a visão + buscar + abrir + importar (sem IA).
- **Fatia B** — ranking por objetivo com IA em cima da visão.

---

## Fatia A — Visão de Issues

### Componentes

**1. Nova visão `View::Issues`**
- Terceira visão além de List/Archive. Tecla `I` alterna para ela (e volta).
- Os arrays por-view (`view_cursor`, `view_scroll`) passam de `[_; 2]` para
  `[_; 3]`; `View::idx()` ganha `Issues => 2`.
- Mostra as issues abertas do repo vinculado ao **projeto em foco**
  (`app.filter.project`). Sem projeto em foco (ou sem vínculo) → mensagem
  orientando a focar um projeto vinculado.

**2. Estado no App**
- `issues: Vec<IssueRow>` (cache da sessão) + `issues_repo: Option<String>`
  (de qual repo/projeto o cache é). `IssueRow { number: u64, title: String,
  url: String, tier: Option<u8>, why: Option<String> }` (tier/why preenchidos
  na Fatia B).
- `issues_cursor: usize` próprio da visão.

**3. Buscar (github.rs)**
- Nova `fetch_issues(repo) -> Result<Vec<IssueRow>>` via
  `gh issue list --repo R --state open --json number,title,url --template ...`
  (número\ttítulo\turl por linha; parser puro).
- Busca ao entrar na visão se o cache é de outro repo; tecla `r` re-busca.
- Falha do `gh` → flash de aviso, mantém o que houver.

**4. Render (ui/issues.rs)**
- Cabeçalho reusando `header` (título = `+projeto · N issues`).
- Cada linha: `#<nº>  <título>` (truncado à largura). Cursor destacado com `▸`.
- Na Fatia B ganha o marcador de tier + linha de porquê.

**5. Ações (só na visão Issues)**
- `j`/`k` move o cursor; `r` re-busca; `Enter` abre a issue no navegador
  (`attach::open_with_system(Path::new(&url))`); `+` importa a issue
  selecionada para o todo.txt (`<título> +<projeto> gh:<repo>#<nº>` via
  `store.add_line`), com flash de confirmação; `I`/`Esc`/`l` volta à Lista.

### Testes (Fatia A)
- Parser puro de `número\ttítulo\turl` → `Vec<IssueRow>` (ignora malformadas).
- Linha de import montada corretamente a partir de um `IssueRow` + projeto.
- `View::idx()` e o resize dos arrays (round-trip de troca de visão).

---

## Fatia B — Ranking por objetivo

### Componentes

**1. Objetivo (config, híbrido)**
- `advisor_goal.<projeto> = "<texto>"` no config (padrão salvo por projeto).
- Sobreposição na hora: tecla `o` na visão abre um campo de texto (draft) para
  digitar um objetivo só para o próximo ranking.

**2. Ranquear (tecla `p`)**
- Monta prompt: objetivo + lista de issues (nº + título). Pede à IA, para cada
  issue: `tier` (1–3) e `why` (uma linha), em formato estruturado parseável
  (`#<nº> <tier> <why>` por linha, ou JSON mínimo).
- Novo parser em `advisor` que casa cada linha de resposta ao `IssueRow` pelo
  número, preenchendo `tier`/`why`.
- Reordena os `issues` por tier (desc) e mantém o cache até novo `p`.

**3. Render do tier**
- Símbolo por contagem: tier 3 = `!!!`, 2 = `!!`, 1 = `!` (helper puro,
  testável), à esquerda do título; a linha de `why` abaixo em cor `dim`.

### Testes (Fatia B)
- Parser da resposta estruturada → (nº → tier, why); casa ao IssueRow certo.
- Helper do símbolo de tier (3→`!!!`, 2→`!!`, 1→`!`, none→vazio).
- Reordenação por tier estável.

---

## Fora de escopo (YAGNI)

- Escrever/fechar issues no GitHub.
- Paginação além do limite do `gh` (`--limit` padrão).
- Múltiplos repos por projeto (segue um repo por projeto).
- Sincronização automática/background (fetch é sob demanda).
