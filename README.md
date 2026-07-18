# Prumo

**Prumo** é um app de tarefas para te manter no prumo — pensado para
cérebros TDAH/TEA, com linguagem natural em português brasileiro, captura
rápida global (`⌥]`) e resumo na barra de menu do macOS. É um fork do
[tuxedo](https://github.com/webstonehq/tuxedo): uma TUI rápida para
[todo.txt](http://todotxt.org/), com atalhos vim, escrita atômica e temas —
tudo em um único binário estático.

> O núcleo Rust mantém o nome `tuxedo` nos caminhos de configuração
> (`~/.config/tuxedo/`) para que as atualizações do upstream continuem
> baratas; o comando instalado e tudo o que você vê na tela usam o nome
> **prumo**.

[![CI](https://github.com/wolffness/prumo/actions/workflows/ci.yml/badge.svg)](https://github.com/wolffness/prumo/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/wolffness/prumo?logo=github)](https://github.com/wolffness/prumo/releases/latest)
[![Licença: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#licença)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg?logo=rust)](https://www.rust-lang.org)

![demo do prumo](docs/demo.gif)

Para um passeio mais aprofundado (no tuxedo original, em inglês), veja
[este vídeo](https://www.youtube.com/watch?v=mT1tg6SQ_Ag) de
[@IogaMaster](https://github.com/IogaMaster).

## Sobre este fork

Este fork ([wolffness/prumo](https://github.com/wolffness/prumo)) adiciona
um conjunto de recursos de fluxo pessoal sobre o
[webstonehq/tuxedo](https://github.com/webstonehq/tuxedo). Tudo nesta seção
é exclusivo do fork; a documentação do upstream que vem depois continua
valendo.

- **Painel de nota no app (`m`).** Veja e edite a nota Markdown de uma
  tarefa em um popup sem sair da TUI: modos visualizar/inserir no estilo
  vim, movimento do cursor por linha visual em texto quebrado, seleção com
  Shift+setas com apagar/substituir, salvamento automático ao fechar. `o`
  continua abrindo no `$EDITOR`.
- **Subtarefas.** Linhas de checkbox (`- [ ]` / `- [x]`) dentro da nota de
  uma tarefa são subtarefas: `n` no painel inicia uma, Enter continua a
  lista, Espaço ou clique do mouse alterna (no painel *e* no painel
  DETAIL). A lista mostra um selo âmbar `[feitas/total]` e o painel DETAIL
  uma barra de progresso âmbar. Subtarefas concluídas aparecem esmaecidas e
  riscadas.
- **Anexos de arquivo (`t`).** Arraste um arquivo para o prompt (ou digite
  um caminho) e ele é *movido* para um diretório `assets/` ao lado do
  arquivo de tarefas, com um token `at:<nome>` anexado à tarefa. Clique no
  nome do arquivo no painel DETAIL (ou pressione Enter na tarefa) para
  abri-lo.
- **Registro de conclusões.** `x` grava um token `done_at:YYYY-MM-DDTHH:MM`
  e arquiva a tarefa em `done.txt` imediatamente; a visão de arquivo (`a`)
  respeita a busca `/` e os filtros de projeto/contexto, então conclusões
  passadas continuam encontráveis.
- **Linguagem natural em português.** O parser de linguagem natural entende
  pt-BR além do inglês: `amanhã`, `hoje`, nomes de dias da semana,
  `para/até sexta`, `em 3 dias`, `15 de junho`, `toda sexta`,
  `a cada 15 dias`, `todo dia 2` (mensal ancorado em um dia),
  `quinzenalmente`, e mais.
- **Tema Phosphor Green.** Um tema monocromático estilo CRT vem embutido e
  é o padrão de fábrica; os acentos âmbar clássicos marcam o progresso das
  subtarefas. Em fundos de cursor claros a linha selecionada inverte o
  vídeo.
- **Bundle de app para macOS.** `./scripts/package-macos.sh` compila e
  instala o `/Applications/Prumo.app` — ícone CRT em pixel art, abre no
  iTerm2 (ou Terminal.app) com um perfil fósforo em IBM Plex Mono, respeita
  `TODO_FILE`/`TODO_DIR` do seu shell de login.
- **Captura rápida nativa (`⌥]`).** Um pequeno agente AppKit (instalado
  pelo script de empacotamento, iniciado no login) mostra um painel
  flutuante no estilo OmniFocus a partir de qualquer app; as entradas caem
  no `inbox.txt` vizinho e fluem para a lista pelo dreno de inbox
  existente, linguagem natural inclusa. Digitar `+` ou `@` abre um popup
  de autocomplete com os projetos/contextos já usados (↑/↓ navega, Tab ou
  Enter aceita). `⌥[` abre o Prumo — ou traz a janela existente à frente.
- **Resumo na barra de menu.** O mesmo agente coloca um ícone na barra de
  menu do macOS mostrando quantas tarefas com data você tem, por urgência:
  `⚠ N` (atrasadas) em laranja, `● N` (para hoje), `○ N` (próximas), ou
  `✓` quando não há nada pendente. O estado é carregado pelo símbolo, não
  só pela cor, e as cores são de alto contraste — legível e seguro para
  daltônicos. O dropdown agrupa as tarefas em **ATRASADAS / HOJE /
  PRÓXIMAS**; clique no círculo de uma linha para concluí-la (`prumo done`)
  ou no texto para abrir o app. Lê a lista de `prumo ls --json`, então o
  núcleo Rust continua sendo a única fonte de verdade.

| Subtarefas: selo âmbar `[2/4]` na lista, barra de progresso + checkboxes clicáveis no DETAIL | O painel de nota no app (`m`) sobre a mesma tarefa |
| --- | --- |
| ![subtarefas do fork](docs/screenshots/fork-subtasks.svg) | ![painel de nota do fork](docs/screenshots/fork-note-panel.svg) |

## Destaques

- **todo.txt puro.** Lê e escreve o [formato padrão](https://github.com/todotxt/todo.txt) — cada linha é texto puro que você pode editar com qualquer outra ferramenta.
- **TUI e CLI em um binário só.** Rode `prumo` para a interface interativa, ou `prumo <comando>` para uma linha de comando compatível com o [todo.txt-cli](https://github.com/todotxt/todo.txt-cli) (`add`, `ls`, `do`, `pri`, `archive`, …) — scriptável, com saída `--json` e suporte a `$TODO_DIR` / `$TODO_FILE` / `$DONE_FILE`.
- **Adição em linguagem natural.** Digite prosa no prompt de adição — `Pagar aluguel todo mês no dia primeiro, mostrar 3 dias antes do vencimento, projeto casa` — e o prumo reescreve em todo.txt canônico para você revisar e salvar. Local, offline, sem serviço de IA.
- **Captura pelo celular.** Pressione `s` para um QR apontando para um mini PWA na sua rede local — digite tarefas do celular e elas aparecem na lista. As capturas caem primeiro no `inbox.txt` vizinho, então qualquer ferramenta que consiga anexar uma linha (shell, Atalhos do iOS, cron) também é uma fonte de captura.
- **Teclas do vim, sem surpresas.** `j` / `k` para mover, `dd` para apagar, `gg` / `G` para saltar, `u` para desfazer (50 níveis), acordes (`gg`, `dd`, `fp`, `fc`) com janela de 600 ms.
- **Paleta de comandos.** `:` ou `Ctrl-P` abre uma paleta com busca difusa sobre todas as ações — digite algumas letras e Enter. Mesmo matcher da busca `/`, ranqueado para que acertos no início do rótulo vençam acertos em fronteira de palavra, que vencem acertos no meio.
- **Escrita atômica, amigável à sincronização.** Toda mudança passa por escrever-temporário-e-renomear. Se outro processo — Dropbox, um editor, um script — modificar o arquivo, o prumo recarrega no próximo toque de tecla (ou em ~250 ms quando ocioso) e pisca um aviso.
- **Arquivo vizinho de concluídas.** `A` move as tarefas concluídas para o `done.txt` ao lado do seu arquivo, atomicamente.
- **Filtro, ordenação, seleção múltipla.** Alterne por `+projeto` ou `@contexto`, ordene por prioridade / vencimento / ordem do arquivo, e conclua ou apague em lote no modo visual.
- **Buscas salvas.** Dê um nome à busca `/` ativa com `fs` e recupere-a a qualquer momento alternando filtros salvos com `ff`. Armazenadas como linhas `filter.<nome>` na configuração — editáveis à mão como todo o resto.
- **Cinco temas, três densidades.** Alterne com `T` e `D`. As escolhas persistem entre execuções e recarregam a quente quando você edita o `config.toml` por fora.
- **Sem daemon, sem banco de dados, sem nuvem.** Um arquivo entra, um arquivo sai.

## Telas

| | |
| --- | --- |
| **Estado vazio** • marca cell-bowtie e guia rápido quando o arquivo não tem tarefas | ![vazio](docs/screenshots/empty.svg) |
| **Lista** • lista de tarefas, opcionalmente agrupada | ![lista](docs/screenshots/list.svg) |
| **Arquivo** • tarefas concluídas agrupadas por data de conclusão | ![arquivo](docs/screenshots/archive.svg) |
| **Sidebar de filtro ativa** • `fp` alterna projetos com j/k, `fc` alterna contextos; buscas salvas listadas sob o título **SAVED** com contagem de resultados ao vivo | ![filtro](docs/screenshots/filter.svg) |
| **Paleta de comandos** • `:` ou `Ctrl-P` abre uma paleta difusa sobre todas as ações | ![paleta de comandos](docs/screenshots/command-palette.svg) |
| **Ajuda** • `?` abre o overlay com todos os atalhos | ![ajuda](docs/screenshots/help.svg) |

<details>
    <summary>Como gerar as capturas de tela e a demo</summary>
    <p>As capturas na tabela acima são SVGs versionados. Regenere-as com:</p>
    <pre>mise run screenshots</pre>
    <p>O GIF do topo é gravado com <a href="https://github.com/charmbracelet/vhs">vhs</a> a partir de <code>docs/demo.tape</code>. Regenere-o com:</p>
    <pre>mise run demo</pre>
</details>

## Temas

`T` abre um seletor com cinco temas embutidos, incluindo o Terminal, que
respeita a paleta do seu terminal.

| Muted Slate (padrão do upstream) | Dawn |
| --- | --- |
| ![muted slate](docs/screenshots/theme-muted-slate.svg) | ![dawn](docs/screenshots/theme-dawn.svg) |
| **Nord** | **Matrix** |
| ![nord](docs/screenshots/theme-nord.svg) | ![matrix](docs/screenshots/theme-matrix.svg) |

### Temas personalizados

Além dos embutidos, o prumo carrega qualquer arquivo `*.toml` que você
colocar em `${XDG_CONFIG_HOME:-$HOME/.config}/tuxedo/themes/`. Cada um entra
no seletor `T` em ordem alfabética de nome de arquivo. Temas prontos vivem
em [`docs/themes/`](docs/themes) — copie um e pressione `T`:

```sh
mkdir -p ~/.config/tuxedo/themes
curl -o ~/.config/tuxedo/themes/gruvbox-dark-soft.toml \
  https://raw.githubusercontent.com/wolffness/prumo/main/docs/themes/gruvbox-dark-soft.toml
```

<details>
<summary>Formato do arquivo de tema e referência de campos</summary>

Um arquivo de tema é um `chave = valor` por linha. `name` é o rótulo exibido
no seletor; todos os outros campos são valores de cor. Todos os campos são
obrigatórios: um arquivo sem algum deles, com uma cor não interpretável, ou
cujo `name` colida com outro tema é ignorado com um aviso na inicialização.

**Valores de cor** aceitam duas formas:

- `#rrggbb` — uma cor hexadecimal sólida (sem distinção de maiúsculas).
- `reset` ou `transparent` — herda a cor de fundo do próprio emulador de
  terminal. Útil em `bg`, `panel` e `statusbar` quando você quer que a
  opacidade, o desfoque ou o papel de parede do seu terminal apareçam,
  mantendo uma paleta de texto personalizada. As duas palavras-chave são
  equivalentes (mesmo efeito do tema embutido **Terminal**).

| Campo | Cores |
| --- | --- |
| `name` | rótulo exibido no seletor `T` (o único campo que não é cor) |
| `bg` | fundo da janela |
| `panel` | fundo dos painéis de filtro e detalhe |
| `border` | bordas de painéis e modais |
| `fg` | texto principal |
| `dim` | texto secundário / esmaecido |
| `accent` | logo, títulos, dicas e marcadores de seleção |
| `cursor` | linha atual, e a linha destacada no seletor `T` |
| `selection` | use o mesmo valor de `selected` |
| `statusbar` | fundo da barra de status |
| `status_fg` | texto da barra de status |
| `mode_fg` / `mode_bg` | texto / fundo do chip de modo |
| `pri_a` `pri_b` `pri_c` `pri_d` | prioridades A a D |
| `pri_other` | prioridades E a Z |
| `project` | tags `+projeto` |
| `context` | tags `@contexto` |
| `due` | data `due:` |
| `overdue` | data vencida |
| `today` | data que vence hoje |
| `done` | tarefas concluídas |
| `selected` | fundo da linha selecionada (modo visual) e do filtro ativo |
| `matched` | destaque de resultado de busca |

</details>

## Instalação

### Homebrew (macOS, Linux)

```sh
brew tap wolffness/prumo
brew trust wolffness/prumo   # tap pessoal — confirmação única
brew install prumo
```

Instala o comando `prumo` (TUI + CLI). Para a experiência completa no macOS
(Prumo.app, captura rápida `⌥]`, barra de menu) veja
[Bundle de app para macOS](#bundle-de-app-para-macos) abaixo.

### A partir do código-fonte

```sh
git clone https://github.com/wolffness/prumo
cd prumo
cargo build --release
./target/release/prumo [ARQUIVO]   # o binário tuxedo também é gerado
```

Requer a edição 2024 do Rust (toolchain estável recente).

### Bundle de app para macOS

```sh
./scripts/package-macos.sh
```

Compila e instala o `/Applications/Prumo.app`: um bundle acessível pelo
Dock/Spotlight que abre a TUI em uma janela de terminal dedicada. O launcher
roda no seu shell de login, então `TODO_FILE`/`TODO_DIR` dos seus dotfiles
são respeitados; sem nenhum dos dois, inicia em `$HOME`.

## Uso

`prumo` é duas coisas em um binário: uma TUI interativa e uma linha de
comando de execução única. Sem subcomando, abre a TUI; com um subcomando
reconhecido, roda a [linha de comando](#interface-de-linha-de-comando) e sai.

```sh
prumo [ARQUIVO]   # abre a TUI no ARQUIVO (criado se não existir)
prumo             # TUI no arquivo padrão (veja a resolução abaixo)
prumo --sample    # abre o arquivo de exemplo no diretório temporário
prumo <comando>   # roda um comando CLI — veja "Interface de linha de comando"
prumo update      # mostra instruções de atualização para a sua instalação
prumo --help
prumo --version
```

Quando há uma versão mais nova disponível, a barra de status mostra
`↑ <versão> (prumo update)` ao lado da versão. A verificação roda em segundo
plano, é cacheada em `$XDG_CACHE_HOME/tuxedo/latest_version.json` por 24 h e
falha silenciosamente quando offline. Defina `TUXEDO_NO_UPDATE_CHECK=1` para
desativar.

### Qual arquivo o prumo abre

Tanto a TUI quanto a CLI resolvem o arquivo de tarefas do mesmo jeito, nesta
ordem:

1. Um argumento `ARQUIVO` explícito (só na TUI).
2. `$TODO_FILE`, se definido.
3. `$TODO_DIR/todo.txt`, se `$TODO_DIR` estiver definido.
4. `./todo.txt` no diretório atual, se existir.
5. Caso contrário a TUI mostra um prompt de primeira execução — pressione
   `c` para criar `./todo.txt` aqui, ou `s` para abrir um todo.txt de
   exemplo no diretório temporário do sistema e explorar sem se comprometer
   com um caminho. (A CLI é não-interativa e usa o exemplo diretamente.)

O arquivo de concluídas é `$DONE_FILE` se definido, senão um `done.txt`
vizinho ao arquivo de tarefas. O arquivo (e diretórios pais que faltem) é
criado no primeiro uso. São as mesmas variáveis `TODO_DIR` / `TODO_FILE` /
`DONE_FILE` do todo.txt-cli, então um `todo.cfg` existente funciona como
está:

```sh
export TODO_DIR="$HOME/Documents/todo"
export TODO_FILE="$TODO_DIR/todo.txt"
export DONE_FILE="$TODO_DIR/done.txt"
```

As edições são persistidas a cada mudança por escrita atômica (escreve
`.tmp`, renomeia).

Se o arquivo mudar no disco (outro editor, um cliente de sincronização, um
script), o prumo percebe no próximo toque de tecla, ou em ~250 ms quando
ocioso, e recarrega. A tecla que disparou o recarregamento é consumida —
pressione de novo para agir sobre o estado atualizado — e a barra de status
pisca um aviso.

Pressionar `A` anexa todas as tarefas concluídas a um `done.txt` vizinho e
as remove do arquivo de trabalho (atomicamente: o `done.txt` é escrito antes
de as originais serem removidas). `a` alterna a visão de arquivo para você
navegar, desarquivar ou apagar permanentemente tarefas passadas.

## Interface de linha de comando

Quando o primeiro argumento é um subcomando reconhecido, o prumo roda um
comando único em vez de abrir a TUI. A superfície espelha o
[todo.txt-cli](https://github.com/todotxt/todo.txt-cli/wiki/Usage) — mesmos
comandos, apelidos, numeração de tarefas e saída — então é um substituto
direto para scripts e aliases.

```sh
prumo add "Pagar aluguel +casa @banco due:2026-07-01"   # ou: prumo a "..."
prumo ls @banco                                          # filtra por contexto
prumo do 3                                               # conclui a tarefa 3
prumo pri 3 A                                            # define prioridade
prumo archive                                            # move concluídas para done.txt
prumo ls --json | jq .                                   # saída legível por máquina
```

| Comando | Apelidos | Argumentos | Descrição |
| --- | --- | --- | --- |
| `add` | `a` | `TEXTO...` | Adiciona uma tarefa (datas em linguagem natural funcionam, igual ao prompt `n`). |
| `append` | `app` | `N TEXTO...` | Anexa texto ao final da tarefa `N`. |
| `prepend` | `prep` | `N TEXTO...` | Insere texto no início da tarefa `N`. |
| `replace` | | `N TEXTO...` | Substitui a tarefa `N` inteira. |
| `pri` | `p` | `N PRIORIDADE` | Define prioridade `A`–`Z` na tarefa `N`. |
| `depri` | `dp` | `N...` | Remove a prioridade das tarefas dadas. |
| `do` | `done`, `complete` | `N...` | Conclui tarefas (recorrentes geram a próxima instância). |
| `del` | `rm` | `N [TERMO]` | Apaga a tarefa `N`, ou remove só `TERMO` dela. Confirma, a menos que `-f`. |
| `archive` | | | Move tarefas concluídas para o arquivo de concluídas. |
| `list` | `ls` | `[TERMO...]` | Lista tarefas. `TERMO` é `+projeto`, `@contexto` ou texto livre. |
| `listall` | `lsa` | `[TERMO...]` | Lista o arquivo de tarefas e o de concluídas. |
| `listpri` | `lsp` | `[PRIORIDADE]` | Lista tarefas com prioridade (opcionalmente uma só). |
| `listproj` | `lsprj` | | Lista todos os `+projetos`. |
| `listcon` | `lsc` | | Lista todos os `@contextos`. |

**Números de tarefa** são números de linha do arquivo, começando em 1,
exatamente como o `list` imprime — estáveis independentemente de filtro ou
ordenação. O `list` ordena pela linha completa (sem distinguir maiúsculas) e
imprime um rodapé `TODO: X of Y tasks shown`, igual ao todo.txt-cli.

**Opções:**

- `-f`, `--force` — pula prompts de confirmação (ex.: no `del`).
- `--json` — emite JSON legível por máquina em vez de texto. Comandos de
  listagem imprimem um array de objetos de tarefa; comandos que modificam
  imprimem um objeto de resultado. Nenhum prompt ou rodapé é escrito nesse
  modo.

Flags globais podem vir antes do subcomando (`prumo -f del 3`).

**Diferenças em relação ao todo.txt-cli:** `do` conclui a tarefa mas **não**
arquiva automaticamente — as concluídas ficam no arquivo até você rodar
`archive` (ou pressionar `A` na TUI), seguindo o modelo interativo do prumo.
Não há a flag `-d` de arquivo de configuração; configure os caminhos com as
variáveis de ambiente acima.

## Atalhos de teclado

Atalhos personalizados do modo normal podem ser adicionados em
`${XDG_CONFIG_HOME:-$HOME/.config}/tuxedo/keybinds.toml`:

O bloco abaixo lista todas as ações remapeáveis com a tecla de fábrica —
copie-o, mude as teclas que interessam e apague o resto (o que você omitir
mantém o padrão). Um valor é uma tecla única ou um array de alternativas,
ex.: `begin_add = ["N", "Ctrl-n"]`.

```toml
[normal]

# Navegação
cursor_down    = ["j", "Down"]
cursor_up      = ["k", "Up"]
cursor_top     = "gg"
cursor_bottom  = "G"
half_page_down = "Ctrl-d"
half_page_up   = "Ctrl-u"

# Edição
begin_add            = "n"
begin_edit           = "e"
begin_edit_insert    = "i"
toggle_complete      = "x"
delete               = "dd"
reschedule           = "r"
cycle_priority       = "p"
begin_prompt_context = "c"
copy_line            = "yy"
copy_body            = "yb"
undo                 = "u"
# begin_prompt_project tem "+" como padrão, que não pode ser escrito aqui (o
# parser lê "+" como separador de modificador). Escolha outra tecla, ex.:
# begin_prompt_project = "P"

# Filtro, ordenação, visão
begin_search        = "/"
arm_f               = "f"        # líder dos acordes fp / fc / ff / fs
pick_project        = "fp"
pick_context        = "fc"
pick_saved_filter   = "ff"
save_current_filter = "fs"
cycle_sort          = "S"
toggle_visual       = "v"
toggle_selected     = "space"
go_list             = "l"
toggle_archive_view = "a"
archive_completed   = "A"
toggle_show_done    = "H"
toggle_show_future  = "F"

# Layout e tema
toggle_left_pane  = "["
toggle_right_pane = "]"
open_theme_picker = "T"
cycle_density     = "D"
toggle_line_num   = "L"
# cycle_theme não tem padrão — vincule uma tecla para alternar temas sem o
# seletor:
# cycle_theme = "Ctrl-t"

# Sistema
open_command_palette = [":", "Ctrl-P"]
open_share           = "s"
open_help            = "?"
open_settings        = ","
escape_stack         = "Esc"
quit                 = "q"
```

Atalhos personalizados são verificados antes dos padrões. Os padrões
continuam disponíveis, a menos que a mesma tecla ou acorde de duas teclas
seja vinculado a outra ação no arquivo. Nomes de ação são snake_case,
seguindo os nomes da paleta de comandos quando possível: `toggle_complete`,
`pick_project`, `open_theme_picker`, e assim por diante. Nomes de tecla
podem ser caracteres únicos, acordes de duas teclas como `ZZ`, formas com
modificador como `Ctrl-n` / `Alt-x`, teclas nomeadas como `Esc`, `Enter`,
`Tab`, setas, `Page-Up`, `Page-Down`, ou `F1` a `F24`.

### Navegação

| Tecla | Ação |
| --- | --- |
| `j` / `↓` | próxima tarefa |
| `k` / `↑` | tarefa anterior |
| `gg` | primeira tarefa |
| `G` | última tarefa |
| `Ctrl-d` / `Ctrl-u` | meia página para baixo / cima |

### Edição

| Tecla | Ação |
| --- | --- |
| `n` | adicionar tarefa |
| `e` | editar a tarefa atual em modo Normal (veja [Diálogo de edição](#diálogo-de-edição)) |
| `i` | editar a tarefa atual em modo Inserir (veja [Diálogo de edição](#diálogo-de-edição)) |
| `x` | concluir: grava `done_at:` e arquiva em `done.txt` |
| `dd` | apagar tarefa |
| `p` | alternar prioridade A → B → C → · |
| `c` | adicionar ou remover um contexto |
| `+` | adicionar um projeto |
| `yy` | copiar a linha atual para a área de transferência |
| `yb` | copiar só o corpo (sem prioridade, datas, projetos, contextos, `chave:valor`) |
| `u` | desfazer (50 níveis) |

### Diálogo de edição

O diálogo de edição usa edição modal no estilo vim. Pressione `i` para
editar a tarefa atual começando em **modo Inserir** — digite imediatamente.
Pressione `e` para começar em **modo Normal** e navegar antes de mudar
qualquer coisa. O prompt de adição (`n`) também abre direto em modo Inserir.

As teclas modais abaixo valem no modo Normal:

| Tecla | Ação |
| --- | --- |
| `h` / `←` | mover cursor para a esquerda |
| `l` / `→` | mover cursor para a direita |
| `w` | saltar para o início da próxima palavra |
| `b` | saltar para o início da palavra anterior |
| `e` | saltar para o fim da palavra atual |
| `x` | apagar o caractere sob o cursor |
| `dw` | apagar até o início da próxima palavra |
| `cw` | apagar até o início da próxima palavra e entrar em modo Inserir |
| `i` | entrar em modo Inserir antes do cursor |
| `a` | entrar em modo Inserir depois do cursor |
| `A` | entrar em modo Inserir no fim da linha |
| `Esc` (no Inserir) | voltar ao modo Normal |
| `Esc` (no Normal) | cancelar e fechar |
| `Enter` (em ambos) | salvar |

### Filtro, ordenação, visão

| Tecla | Ação |
| --- | --- |
| `/` | buscar |
| `fp` | filtrar por projeto (`j` / `k` alterna, `Esc` limpa) |
| `fc` | filtrar por contexto (`j` / `k` alterna, `Esc` limpa) |
| `ff` | escolher uma busca salva (`j` / `k` alterna, `Enter` mantém, `Esc` reverte) |
| `fs` | salvar a busca `/` ativa como filtro nomeado |
| `S` | alternar ordenação: prioridade → vencimento → ordem do arquivo |
| `v` | entrar no modo visual / seleção múltipla; `espaço` alterna uma linha |
| `x` / `dd` (no visual) | concluir / apagar a seleção em lote |
| `l` | visão de lista (padrão) |
| `a` | alternar visão de arquivo |
| `A` | arquivar tarefas concluídas → `done.txt` |
| `H` | alternar exibição de concluídas na lista principal |
| `o` | abrir o `note:<caminho>` existente da tarefa no `$VISUAL` / `$EDITOR` |
| `O` | criar a nota da tarefa se preciso, depois abri-la |
| `m` / `N` | abrir a nota em um painel no app (ver + editar, salva ao fechar) |
| `Espaço` / `n` (no painel) | alternar / iniciar um checkbox de subtarefa |
| `t` | anexar um arquivo: arraste-o para o prompt ou digite um caminho (movido para `assets/`) |
| `Enter` | abrir os anexos `at:` da tarefa atual com o abridor do sistema |

### Layout e tema

| Tecla | Ação |
| --- | --- |
| `[` | alternar sidebar de filtro |
| `]` | alternar sidebar de detalhe |
| `T` | abrir seletor de temas |
| `D` | alternar densidade: compacta → confortável → espaçosa |
| `L` | alternar números de linha |

### Sistema

| Tecla | Ação |
| --- | --- |
| `:` / `Ctrl-P` | paleta de comandos |
| `s` | QR de captura (PWA do celular) |
| `?` | overlay de ajuda |
| `,` | overlay de configurações |
| `q` | sair |

Prompts de acorde de duas teclas (`gg`, `dd`, `yy`, `yb`, `fp`, `fc`, `ff`,
`fs`) mostram um indicador `g…` / `d…` / `y…` / `f…` no chip de modo da
barra de status enquanto o líder está armado; a janela é de 600 ms.

A cópia usa o escape de terminal OSC 52, então funciona localmente e por SSH
em qualquer terminal com suporte (kitty, alacritty, wezterm, iTerm2, foot,
xterm moderno; tmux com `set -g set-clipboard on`). Terminais mais antigos
ignoram a tecla silenciosamente.

## Formato todo.txt

Linhas [todo.txt](https://github.com/todotxt/todo.txt) padrão:

```
(A) 2026-04-28 Ligar para o dentista @telefone +saude due:2026-05-08
```

- `(A)` — prioridade, A a Z (omita para nenhuma)
- `2026-04-28` — data de criação em ISO 8601
- `+projeto` — tag de projeto
- `@contexto` — tag de contexto
- `chave:valor` — extensão; `due:YYYY-MM-DD` é reconhecida para ordenação e
  agrupamento por vencimento na visão de lista. `note:<caminho>` é
  reconhecida pelas ações de nota (`o` / `O`): caminhos relativos resolvem
  sob `notes_dir`, depois `$NOTES_DIR`, depois `~/notes`. Chaves que você
  prefere não ver podem ser ocultadas das linhas via
  [`hide_keys`](#ocultando-tags-chavevalor)
- `rec:[+]N{d,b,w,m,y}` — recorrência; ao concluir (`x`), o prumo insere
  uma cópia nova da tarefa com o `due:` avançado em `N` dias, dias úteis
  (seg–sex), semanas, meses ou anos. O prefixo `+` significa recorrência
  *estrita*, ancorada no vencimento anterior (ex.: `rec:+1m` para o aluguel
  mensal do dia 15); sem ele, o novo vencimento é calculado a partir da
  data de conclusão (ex.: `rec:1w` para "regar as plantas uma semana depois
  da última vez").

Tarefas concluídas são prefixadas com `x ` e a data de conclusão:

```
x 2026-05-05 2026-05-01 Enviar relatório de despesas +trabalho
```

Exemplo com recorrência:

```
2026-05-09 Pagar aluguel due:2026-05-15 rec:+1m
```

Pressionar `x` na linha acima conclui a original *e* insere
`2026-05-09 Pagar aluguel due:2026-06-15 rec:+1m`. `u` desfaz as duas de uma
vez.

## Adição em linguagem natural

Pressione `n` para abrir o prompt de adição. Digite a tarefa em prosa —
português ou inglês. Quando o texto contém frases reconhecidas (datas, dias
da semana, recorrência, nomes de projeto / contexto, prioridade), Enter
reescreve o rascunho em todo.txt canônico — revise ou ajuste, e Enter de
novo para salvar.

| O que você digita | O que entra no rascunho |
| --- | --- |
| `Comprar leite amanhã` | `Comprar leite due:2026-07-19` |
| `Pagar aluguel todo dia 15` | `Pagar aluguel due:2026-08-15 rec:+1m` |
| `Enviar timesheet a cada 15 dias` | `Enviar timesheet rec:+15d` |
| `Regar plantas toda sexta` | `Regar plantas due:2026-07-24 rec:+1w` |
| `Comprar tinta projeto casa e contexto loja` | `Comprar tinta +casa @loja` |
| `Pagar boleto amanhã mostrar 3 dias antes prioridade alta` | `(A) Pagar boleto due:2026-07-19 t:-3d` |
| `Pay rent monthly on the first, show 3 days before due, project home` | `Pay rent +home due:2026-08-01 rec:+1m t:-3d` |

Vocabulário reconhecido:

- **Datas (pt-BR e inglês)** — `hoje`, `amanhã`, `ontem`, dias da semana (`segunda` / `seg` …), `para/até sexta`, `em 3 dias`, meses (`15 de junho`), ISO `2026-05-15`.
- **Recorrência (pt-BR e inglês)** — `toda sexta`, `todo dia 2` (mensal ancorado no dia), `a cada 15 dias`, `quinzenalmente`, `daily`, `every business day`, e variações.
- **Antecedência em prosa (pt-BR e inglês)** — `mostrar 3 dias antes`, `2 semanas antes do vencimento`, `show 3 days before due`.
- **Projeto / contexto em prosa (pt-BR e inglês)** — `projeto casa`, `contexto banco`, `project home`, `context bank`, ou os sigilos padrão `+casa` / `@banco`.
- **Prioridade em prosa (pt-BR e inglês)** — `prioridade alta` → A, `prioridade média` → B, `prioridade baixa` → C, `prioridade a`, também invertido (`alta prioridade`), e as formas em inglês (`high priority`).

A interpretação é baseada em regras e roda localmente — sem chamadas de
rede, sem chave de API. Se o texto já contém um token `due:`, `rec:` ou
`t:`, o prumo assume que você digitou a forma canônica e salva direto no
primeiro Enter.

## Captura pelo celular

Pressione `s` para iniciar um pequeno servidor de captura no endereço da
sua máquina na rede local e exibir um QR code. Escaneie do celular —
qualquer navegador moderno — para abrir um PWA minimalista que você pode
instalar na tela inicial. Digite uma tarefa, toque em Add, e em instantes
ela aparece na sua lista.

As capturas nunca tocam o `todo.txt` diretamente. Elas caem em um
`inbox.txt` vizinho, que o prumo drena a cada verificação de mudança
externa: cada linha passa pelo mesmo pipeline de linguagem natural do
prompt `n`, ganha data de criação se faltar, e é mesclada no `todo.txt`
como um lote único desfazível (`u` reverte o dreno inteiro de uma vez).

Isso torna o `inbox.txt` um ponto de captura genérico, não só o backend do
PWA. Qualquer coisa que anexe uma linha funciona como produtor:

```sh
echo "Renovar receita amanhã" >> ~/notes/inbox.txt
echo "Ligar para o dentista due:2026-06-01" >> ~/notes/inbox.txt
```

Aliases de shell, Atalhos do iOS escrevendo numa pasta sincronizada, cron,
gateways de e-mail para arquivo — escolha seu produtor. Desde que anexe uma
linha ao `inbox.txt` vizinho, o prumo captura.

O servidor:

- Sobe no primeiro `s` e permanece ativo pelo resto da sessão. Pressionar
  `s` de novo só reexibe o QR; qualquer tecla fecha o overlay.
- Escuta em `0.0.0.0:<porta>` para que celulares no mesmo WiFi alcancem. A
  porta é atribuída pelo SO no primeiro uso e persistida no `config.toml`,
  então favoritos do celular sobrevivem entre sessões.
- Protege todas as rotas com um token hexadecimal de 64 caracteres embutido
  no caminho da URL. O token é gerado uma vez, persistido no `config.toml`
  e comparado em tempo constante.
- Fala HTTP puro — **somente redes confiáveis.** Em um WiFi compartilhado ou
  público, qualquer um farejando passivamente pode recuperar o token. Para
  rotacionar, apague `share_token` do `config.toml` e pressione `s` de novo.

Drenos de produtores gerenciados pelo prumo são à prova de travamento: o
servidor de captura segura a mesma trava consultiva do
renomear-e-mesclar da TUI, e qualquer arquivo de staging deixado por um
dreno interrompido é reexecutado na próxima sessão. Anexos diretos via
shell são úteis para captura leve, mas não tomam essa trava; use o servidor
de captura ou a mesma trava se um produtor precisar ser serializado com o
dreno da TUI.

## Configuração

Persistida em `${XDG_CONFIG_HOME:-$HOME/.config}/tuxedo/config.toml`.
Alternar tema, densidade ou ordenação, e ligar/desligar sidebars, números
de linha e visibilidade de concluídas atualizam o arquivo. Chaves
desconhecidas são ignoradas, então binários antigos não quebram com
arquivos mais novos.

**Recarga a quente.** Edições no `config.toml` são aplicadas com a TUI
rodando — mude tema, densidade, ordenação, layout, filtros salvos ou
qualquer outro campo e a UI atualiza em ~200 ms. Falhas de interpretação
(ex.: um erro de digitação no meio da edição) mantêm a configuração em uso
intacta e piscam um aviso na barra de status.

Duas chaves adicionais, `share_token` e `share_port`, são escritas pelo
servidor de [captura pelo celular](#captura-pelo-celular) no primeiro uso.
Trate `share_token` como segredo — quem tiver o valor e acesso à sua rede
local pode anexar ao seu inbox. Apague a chave do `config.toml` para
rotacioná-la no próximo `s`.

Buscas salvas (criadas com `fs`) são escritas uma por linha como
`filter.<nome> = <consulta>`, onde `<consulta>` é o termo da busca `/`. Elas
round-trip como texto puro, então você pode adicionar, renomear ou apagar
editando o `config.toml` diretamente; um `filter.<nome>` repetido mantém o
último valor, e `<nome>` não pode conter `=`.

As ações de nota resolvem tokens `note:<caminho>` relativos sob `notes_dir`.
Se `notes_dir` não estiver definido, o prumo recorre a `$NOTES_DIR` e depois
a `~/notes`. `O` cria notas ausentes sob `projects/tuxedo-tasks/` usando um
pequeno template Markdown e anexa o token `note:<caminho>` gerado à tarefa;
`o` só abre uma nota já vinculada.

```toml
notes_dir = ~/notes
```

### Ocultando tags `chave:valor`

Algumas extensões `chave:valor` são para máquinas, não para os olhos — ex.:
um `uid:` de sincronização. Adicione uma linha `hide_keys` separada por
vírgulas ao `config.toml` e os tokens dessas chaves somem das linhas de
tarefa (visões de lista e arquivo):

```toml
hide_keys = uid, sync
```

A correspondência não distingue maiúsculas. A ocultação é puramente visual —
as tags permanecem no disco intactas, continuam serializando e ainda
aparecem na seção **RAW** do painel de detalhe (uma válvula de escape
deliberada). Buscas ainda encontram o texto oculto; os caracteres apenas não
são desenhados.

## Desenvolvimento

```sh
mise run fmt      # cargo fmt --all
mise run clippy   # cargo clippy --all-targets --locked -- -D warnings
mise run test     # cargo test --locked
```

O CI roda os três em cada push e pull request. As tarefas também rodam como
comandos `cargo` puros se você não usa o [mise](https://mise.jdx.dev/).

## Agradecimentos

- [todo.txt](http://todotxt.org/) de Gina Trapani — o formato que torna uma ferramenta assim possível.
- [tuxedo](https://github.com/webstonehq/tuxedo) — o upstream sobre o qual o Prumo é construído.
- [ratatui](https://ratatui.rs/) e [crossterm](https://github.com/crossterm-rs/crossterm) — os crates de renderização e entrada de terminal na base do prumo.

## Roadmap

O trabalho planejado e em andamento vive no [`todo.txt`](./todo.txt) — na
prática do próprio formato.

## Contribuindo

Issues e pull requests são bem-vindos (em português ou inglês). Para
mudanças maiores, abra uma issue antes para discutir a abordagem. Rode
`mise run fmt clippy test` (ou os equivalentes em cargo puro) antes de
enviar.

## Licença

Distribuído sob a [Licença MIT](https://opensource.org/licenses/MIT).
