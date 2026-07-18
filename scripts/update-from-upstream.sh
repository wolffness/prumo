#!/bin/zsh
# Atualiza este fork a partir do Tuxedo original (webstonehq/tuxedo)
# preservando todas as personalizacoes locais.
#
# Fluxo, todo com rede de seguranca:
#   1. Exige a arvore de trabalho limpa (nada por commitar)
#   2. Garante o remote 'upstream' e busca as novidades
#   3. Se nao ha nada novo, sai sem fazer nada
#   4. Cria/atualiza a branch de backup 'backup-antes-do-merge'
#   5. Faz o merge. Conflito SO em snapshots (.snap) -> resolve sozinho
#      regenerando-os; conflito em codigo (.rs etc.) -> aborta e avisa
#   6. Regenera snapshots sobre o codigo mesclado e roda a suite completa
#   7. (pergunta) publica no seu GitHub  8. (pergunta) reinstala o .app
#
# Uso:  ./scripts/update-from-upstream.sh [-y]
#   -y / --yes   nao pergunta: ja publica e reinstala o app
set -euo pipefail

cd "$(dirname "$0")/.."

UPSTREAM_URL="https://github.com/webstonehq/tuxedo.git"
BACKUP_BRANCH="backup-antes-do-merge"
AUTO_YES=false

for arg in "$@"; do
  case "$arg" in
    -y|--yes) AUTO_YES=true ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "Opcao desconhecida: $arg (use -h)"; exit 1 ;;
  esac
done

# Pergunta s/N; com -y responde 'sim' automaticamente.
confirm() {
  $AUTO_YES && return 0
  printf "%s [s/N] " "$1"
  read -r resposta
  [[ "$resposta" == [sSyY] ]]
}

# --- 1. Arvore de trabalho precisa estar limpa -----------------------------
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "✋ Voce tem mudancas nao commitadas. Faca commit ou 'git stash' antes."
  git status -sb
  exit 1
fi

# --- 2. Remote upstream + fetch --------------------------------------------
if ! git remote | grep -qx upstream; then
  echo "→ Configurando remote 'upstream' ($UPSTREAM_URL)"
  git remote add upstream "$UPSTREAM_URL"
fi
echo "→ Buscando novidades do original..."
git fetch upstream --quiet

# --- 3. Ha algo novo? ------------------------------------------------------
BEHIND=$(git rev-list --count HEAD..upstream/main)
if [[ "$BEHIND" -eq 0 ]]; then
  echo "✅ Ja esta atualizado — nada novo no original."
  exit 0
fi
echo "→ $BEHIND novo(s) commit(s) no original:"
git log --oneline HEAD..upstream/main | sed 's/^/    /'

# --- 4. Backup -------------------------------------------------------------
git branch -f "$BACKUP_BRANCH" HEAD
echo "→ Backup do estado atual salvo na branch '$BACKUP_BRANCH'."

# --- 5. Merge (sem commitar ainda, para incluir snapshots regenerados) -----
echo "→ Fazendo merge..."
git merge --no-commit --no-edit upstream/main || true

CONFLICTS=$(git diff --name-only --diff-filter=U || true)
if [[ -n "$CONFLICTS" ]]; then
  NON_SNAP=$(echo "$CONFLICTS" | grep -v '\.snap$' || true)
  if [[ -n "$NON_SNAP" ]]; then
    echo "⚠️  Conflito em codigo (nao sao snapshots):"
    echo "$NON_SNAP" | sed 's/^/    /'
    git merge --abort
    echo "→ Merge revertido; seu repo esta intacto (backup em '$BACKUP_BRANCH')."
    echo "   Resolva manualmente ou peca ajuda ao Claude Code neste diretorio."
    exit 1
  fi
  # So snapshots conflitaram: destrava (serao regenerados a seguir)
  echo "→ Conflito apenas em snapshots — resolvendo automaticamente."
  echo "$CONFLICTS" | xargs git checkout --ours --
  echo "$CONFLICTS" | xargs git add --
fi

# --- 6. Regenera snapshots sobre o codigo mesclado + suite completa --------
echo "→ Regenerando snapshots de UI..."
INSTA_UPDATE=always cargo test --test snapshots >/dev/null 2>&1 || true
find tests/snapshots -name '*.snap.new' -delete
git add tests/snapshots/ 2>/dev/null || true

echo "→ Rodando a suite de testes..."
if ! cargo test >/dev/null 2>&1; then
  echo "❌ Os testes falharam apos o merge. Abortando por seguranca."
  git merge --abort 2>/dev/null || git reset --hard "$BACKUP_BRANCH"
  echo "→ Repo restaurado ao estado anterior (backup '$BACKUP_BRANCH')."
  exit 1
fi

# Finaliza o commit de merge (se ainda estiver em curso)
if [[ -f .git/MERGE_HEAD ]]; then
  git commit --no-edit >/dev/null
fi
echo "✅ Merge concluido e testado. Personalizacoes preservadas."

# --- 7. Publicar no seu GitHub ---------------------------------------------
if confirm "Publicar no seu GitHub (git push origin main)?"; then
  git push origin main
  echo "✅ Publicado."
else
  echo "→ Pulei o push. Rode 'git push origin main' quando quiser."
fi

# --- 8. Reinstalar o app com a atualizacao ---------------------------------
if confirm "Reinstalar o app do Prumo com esta atualizacao?"; then
  ./scripts/package-macos.sh
  echo "✅ App reinstalado. Reabra o Prumo pelo icone."
else
  echo "→ Pulei o reempacotamento. Rode './scripts/package-macos.sh' quando quiser."
fi

echo ""
echo "🎉 Pronto. Se algo tiver saido errado, volte com:"
echo "     git reset --hard $BACKUP_BRANCH"
