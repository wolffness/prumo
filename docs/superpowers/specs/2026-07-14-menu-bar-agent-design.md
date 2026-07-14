# Design: ícone de menu bar com resumo de tarefas do dia

Data: 2026-07-14
Status: aprovado (aguardando revisão do spec)

## Propósito

Dar um relance permanente das tarefas do dia na barra de menus do macOS, sem
abrir o app. Um `NSStatusItem` mostra quantas tarefas precisam de atenção hoje
e um dropdown lista atrasadas/hoje, permitindo concluí-las na hora.

## Escopo

- **Inclui**: ícone com contador colorido; dropdown com resumo + lista agrupada
  (atrasadas / hoje) limitada por grupo; concluir tarefa pelo menu; abrir o
  Tuxedo; abrir o painel de captura (⌥]) a partir do menu.
- **Não inclui**: editar texto de tarefa pelo menu; filtros/projetos no menu;
  notificações/alertas; tarefas "a vencer" além de hoje (só resumo atrasadas+hoje
  no v1; "próximas" fica para depois).

## Arquitetura: um único agente

Funde o atual `TuxedoCapture.app` e o novo ícone de menu bar num só agente
`.accessory`, renomeado para **`TuxedoAgent.app`**. Um processo, um LaunchAgent.

Motivo: os dois são agentes persistentes que resolvem `TODO_FILE` e compartilham
a estética phosphor. Fundir permite que o item **"Nova tarefa…"** do menu reuse
diretamente o painel de captura (chamada de método), sem IPC entre processos.

Migração: o `package-macos.sh` passa a instalar `TuxedoAgent.app` +
LaunchAgent `dev.wolffness.tuxedo.agent`, e **remove/descarrega** o antigo
`dev.wolffness.tuxedo.capture` (bootout do LaunchAgent + remoção do `.plist`)
para não deixar dois agentes rodando.

## Componentes (Swift, compilados juntos no bundle)

| Arquivo | Responsabilidade | O que faz |
|---|---|---|
| `Shared.swift` | Base compartilhada | Cores phosphor; `resolveTodoFile()` / `resolveInbox()` (extraído do capture atual) |
| `CapturePanel.swift` | Captura ⌥] | O painel flutuante atual, movido sem mudança de comportamento |
| `MenuBar.swift` | **Novo** | `NSStatusItem`, polling de `tuxedo ls --json`, construção do menu, ações |
| `main.swift` | Bootstrap | Instancia painel + menu bar, registra hotkey, sobe o `NSApplication` |

`package-macos.sh` compila a pasta inteira (`swiftc ... packaging/agent/*.swift`)
em vez de um arquivo único.

## Fluxo de dados (fonte única = binário Rust)

```
todo.txt muda ─────────(DispatchSource file-watch)──┐
menu vai abrir ────────(menuNeedsUpdate)────────────┼─▶ roda `tuxedo ls --json`
vira o dia (meia-noite)─(timer)─────────────────────┘        │
                                                             ▼
                                    filtra por data + monta contagem/estado
                                    atrasada: due < hoje, !done
                                    hoje:     due == hoje, !done
                                                             ▼
                                            atualiza ícone + reconstrói menu
```

O agente **não reimplementa** parsing de todo.txt: chama `tuxedo ls --json`
(campos `n, raw, done, priority, due, projects, contexts, rec, t`) e apenas
filtra por data em Swift. Chamada é barata; roda em mudança de arquivo, ao abrir
o menu e na virada do dia.

## UX

### Ícone (NSStatusItem)
- Título = número de **atrasadas + hoje**.
- Cor: phosphor-green normal; **âmbar (alerta, coerente com CRT) quando há ≥1
  atrasada**. Vermelho fica reservado para um possível estado futuro mais grave.
- Zero pendências: ícone neutro/apagado, sem número.

### Dropdown
```
ATRASADAS
  ☐ (A) Ligar para cliente          −4d
  ☐ Comprar café                    −1d
HOJE
  ☐ (B) Revisar proposta
  ☐ Enviar relatório
  … +N mais
─────────────
Abrir Tuxedo
Nova tarefa…                         ⌥]
```
- Até ~5 itens por grupo; excedente vira "+N mais…" (abre o Tuxedo).
- Cada linha tem uma caixa ☐ e o texto da tarefa; atrasadas mostram "−Nd".

### Interações
- **☐** → conclui a tarefa. Anti-corrida: no clique, re-roda `tuxedo ls --json`,
  localiza a tarefa pelo campo `raw` (não pela posição), pega o `n` atual e chama
  `tuxedo done <n>`. Se não achar (arquivo mudou), só atualiza o menu.
- **Texto da tarefa** → abre o Tuxedo (via o launcher / `open` do app).
- **Nova tarefa…** → mostra o painel de captura (⌥]).
- **Abrir Tuxedo** → abre o app.

## Concorrência com o TUI (resolvido)

Concluir pela barra escreve no `todo.txt` via CLI. O TUI chama
`check_external_changes()` a cada tick e antes de cada mutação: ao detectar a
alteração no disco, recarrega (`Reconcile::Reloaded`, flash
"file changed on disk — reloaded"). Logo, a conclusão feita pela barra é
reconciliada pelo TUI, sem perda de dados, mesmo com o app aberto.

## Refresh

- File-watch (`DispatchSource`) no `todo.txt`.
- `menuNeedsUpdate` recarrega ao abrir o menu (frescor no clique).
- Timer agendado para a próxima meia-noite (recalcula atrasada/hoje na virada).

## Testes

- **Lógica de data pura** (Swift): dado um conjunto de tarefas `(due, done)` e uma
  "data de hoje" injetada, verificar contagem de atrasadas/hoje, cor do ícone e
  agrupamento. Sem tocar em relógio real nem no filesystem.
- **Integração**: `tuxedo ls --json` e `tuxedo done N` exercitados de ponta a
  ponta num `TODO_FILE` de teste (scratchpad).
- Parsing de todo.txt permanece coberto pelos testes Rust existentes.

## Riscos e mitigações

| Risco | Mitigação |
|---|---|
| Concluir a tarefa errada por mudança do arquivo | Casar por `raw` e reconsultar `n` antes do `done` |
| Perder conclusão com TUI aberto | Reconcile do TUI já recarrega do disco (acima) |
| Dois agentes após rename | `package-macos.sh` faz bootout + remove o `.plist` antigo |
| Estado do ícone "velho" | Recarrega no file-watch, no menuNeedsUpdate e à meia-noite |

## Fora deste spec (backlog futuro)
- Grupo "próximas" (a vencer) no dropdown.
- Notificações no vencimento.
- Preferência de qual contagem exibir (hoje só vs. hoje+atrasadas).
