//! Integração GitHub do advisor: puxa issues abertas de repos vinculados a
//! projetos do todo.txt e as transforma em linhas sintéticas para a
//! priorização. A parte pura (parse da saída do `gh` + montagem das linhas)
//! fica isolada do shell-out para os testes rodarem offline.

use std::process::Command;

use anyhow::{Result, anyhow};

/// Parseia a lista de repos vinda de `gh repo list ... --template` (um
/// `owner/repo` por linha). Descarta linhas vazias e apara espaços.
pub fn parse_repo_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parseia a saída `<número>\t<título>` (uma issue por linha) de
/// `gh issue list ... --template`. Linhas sem tab ou com número/título
/// inválido são ignoradas (forgiving).
pub fn parse_issue_tsv(stdout: &str) -> Vec<(u64, String)> {
    stdout
        .lines()
        .filter_map(|line| {
            let (num, title) = line.split_once('\t')?;
            let num = num.trim().parse::<u64>().ok()?;
            let title = title.trim();
            if title.is_empty() {
                return None;
            }
            Some((num, title.to_string()))
        })
        .collect()
}

/// Monta a linha sintética de uma issue, no formato todo.txt:
/// `(?) <título> +<projeto> gh:<owner/repo>#<número>`. O marcador `(?)` e o
/// token `gh:` deixam claro que o item vem do GitHub e não está no todo.txt.
pub fn synthetic_line(project: &str, repo: &str, number: u64, title: &str) -> String {
    format!("(?) {title} +{project} gh:{repo}#{number}")
}

/// Todas as linhas sintéticas para um repo/projeto a partir da saída crua do
/// `gh issue list`.
pub fn synthetic_lines(stdout: &str, project: &str, repo: &str) -> Vec<String> {
    parse_issue_tsv(stdout)
        .into_iter()
        .map(|(n, t)| synthetic_line(project, repo, n, &t))
        .collect()
}

/// Executa um subcomando do `gh` já autenticado, devolvendo o stdout. Isola o
/// shell-out (como o `curl` do incremento 1) para o resto do módulo ficar puro.
fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh").args(args).output().map_err(|e| {
        anyhow!("não encontrei o `gh` no PATH ({e}). Instale o GitHub CLI e rode `gh auth login`.")
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("`gh {}` falhou: {}", args.join(" "), err.trim()));
    }
    String::from_utf8(out.stdout).map_err(|e| anyhow!("saída do gh não é UTF-8: {e}"))
}

/// Lista os repos da conta logada como `owner/repo`, um por linha.
pub fn list_repos() -> Result<Vec<String>> {
    let out = gh(&[
        "repo",
        "list",
        "--limit",
        "100",
        "--json",
        "nameWithOwner",
        "--template",
        "{{range .}}{{.nameWithOwner}}{{\"\\n\"}}{{end}}",
    ])?;
    Ok(parse_repo_list(&out))
}

/// Linhas sintéticas das issues abertas de um repo vinculado ao `project`.
pub fn open_issue_lines(repo: &str, project: &str) -> Result<Vec<String>> {
    let out = gh(&[
        "issue",
        "list",
        "--repo",
        repo,
        "--state",
        "open",
        "--json",
        "number,title",
        "--template",
        "{{range .}}{{.number}}{{\"\\t\"}}{{.title}}{{\"\\n\"}}{{end}}",
    ])?;
    Ok(synthetic_lines(&out, project, repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repo_list_dropping_blanks() {
        let out = "wolffness/prumo\n\n  wolffness/casa-infra  \n";
        assert_eq!(
            parse_repo_list(out),
            vec![
                "wolffness/prumo".to_string(),
                "wolffness/casa-infra".to_string()
            ]
        );
    }

    #[test]
    fn parses_issue_tsv_and_skips_malformed() {
        let out = "12\tArrumar o parser NL\nsem-tab\n7\t  Publicar release  \nx\tnúmero inválido\n";
        assert_eq!(
            parse_issue_tsv(out),
            vec![
                (12, "Arrumar o parser NL".to_string()),
                (7, "Publicar release".to_string())
            ]
        );
    }

    #[test]
    fn builds_synthetic_line() {
        assert_eq!(
            synthetic_line("prumo", "wolffness/prumo", 12, "Arrumar o parser NL"),
            "(?) Arrumar o parser NL +prumo gh:wolffness/prumo#12"
        );
    }

    #[test]
    fn synthetic_lines_maps_all_issues() {
        let out = "12\tA\n7\tB\n";
        assert_eq!(
            synthetic_lines(out, "prumo", "wolffness/prumo"),
            vec![
                "(?) A +prumo gh:wolffness/prumo#12".to_string(),
                "(?) B +prumo gh:wolffness/prumo#7".to_string(),
            ]
        );
    }
}
