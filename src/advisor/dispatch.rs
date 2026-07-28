//! Dispatch de agentes: dado um prompt (vindo do draft de despacho), um
//! agente e um diretório de projeto, dispara `herdr agent start
//! dispatch-<slug>` — o herdr é o terminal do usuário, então a execução
//! nasce visível e supervisionável.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow};

/// Nome do agente herdr para um draft (`dispatch-<slug>`).
pub fn agent_name(slug: &str) -> String {
    format!("dispatch-{slug}")
}

/// Argv do agente: sonnet/opus/fable → `claude --model <x>`; codex → `codex`;
/// sem agente definido → `claude` (modelo default).
pub fn agent_argv(agent: &str, prompt: &str) -> Vec<String> {
    match agent {
        "codex" => vec!["codex".into(), prompt.into()],
        "sonnet" | "opus" | "fable" => vec![
            "claude".into(),
            "--model".into(),
            agent.into(),
            prompt.into(),
        ],
        _ => vec!["claude".into(), prompt.into()],
    }
}

/// Diretório local de um repo `owner/nome`: procura por `nome` (case-
/// insensitive) dentro de `$PRUMO_REPOS_DIR` (default `~/Documents/Projetos`).
pub fn repo_dir(repo: &str) -> Result<PathBuf> {
    let name = repo.rsplit('/').next().unwrap_or(repo);
    let base = std::env::var("PRUMO_REPOS_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home).join("Documents/Projetos")
    });
    find_dir_case_insensitive(&base, name).ok_or_else(|| {
        anyhow!(
            "repo `{repo}` não encontrado em {} (defina PRUMO_REPOS_DIR)",
            base.display()
        )
    })
}

/// Primeiro subdiretório de `base` cujo nome bate com `name` ignorando caixa.
fn find_dir_case_insensitive(base: &Path, name: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(base).ok()?;
    let wanted = name.to_lowercase();
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .map(|f| f.to_string_lossy().to_lowercase() == wanted)
                    .unwrap_or(false)
        })
}

/// Executa um subcomando do `herdr`, devolvendo o stdout (espelha o `gh`).
pub(crate) fn herdr(args: &[&str]) -> Result<String> {
    let out = Command::new("herdr").args(args).output().map_err(|e| {
        anyhow!("não encontrei o `herdr` no PATH ({e}). O dispatch requer o herdr.")
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("`herdr {}` falhou: {}", args.join(" "), err.trim()));
    }
    String::from_utf8(out.stdout).map_err(|e| anyhow!("saída do herdr não é UTF-8: {e}"))
}

/// O agente `dispatch-<slug>` já existe no herdr?
pub fn is_dispatched(slug: &str) -> bool {
    herdr(&["agent", "get", &agent_name(slug)]).is_ok()
}

/// Estado do agente `dispatch-<slug>` no herdr (`working`/`blocked`/`idle`/
/// ...), ou `None` se não existe (ou o herdr está indisponível).
pub fn agent_status(slug: &str) -> Option<String> {
    let out = herdr(&["agent", "get", &agent_name(slug)]).ok()?;
    extract_agent_status(&out)
}

/// Extrai o primeiro `"agent_status":"<x>"` de um JSON do herdr.
pub fn extract_agent_status(json: &str) -> Option<String> {
    let rest = json.split("\"agent_status\":\"").nth(1)?;
    let status = rest.split('"').next()?;
    (!status.is_empty()).then(|| status.to_string())
}

/// Dispara o agente do draft: `herdr agent start dispatch-<slug> --cwd <dir>
/// --no-focus -- <argv do agente>`.
pub fn dispatch(slug: &str, agent: &str, dir: &Path, prompt: &str) -> Result<()> {
    let name = agent_name(slug);
    let argv = agent_argv(agent, prompt);
    let dir_s = dir.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec!["agent", "start", &name, "--cwd", &dir_s, "--no-focus", "--"];
    args.extend(argv.iter().map(|s| s.as_str()));
    herdr(&args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_agents_to_argv() {
        assert_eq!(agent_argv("codex", "p"), vec!["codex", "p"]);
        assert_eq!(agent_argv("opus", "p"), vec!["claude", "--model", "opus", "p"]);
        assert_eq!(agent_argv("", "p"), vec!["claude", "p"]);
        assert_eq!(agent_argv("sem agente", "p"), vec!["claude", "p"]);
    }

    #[test]
    fn finds_dir_ignoring_case() {
        let base = std::env::temp_dir().join(format!("prumo-dispatch-test-{}", std::process::id()));
        let dir = base.join("MeuRepo");
        std::fs::create_dir_all(&dir).unwrap();
        let found = find_dir_case_insensitive(&base, "meurepo").unwrap();
        assert_eq!(found, dir);
        assert!(find_dir_case_insensitive(&base, "outro").is_none());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn agent_name_format() {
        assert_eq!(agent_name("csv-export"), "dispatch-csv-export");
    }

    #[test]
    fn extracts_agent_status_from_json() {
        let json = r#"{"result":{"agent":{"agent_status":"working","name":"dispatch-x"}}}"#;
        assert_eq!(extract_agent_status(json).as_deref(), Some("working"));
        assert_eq!(extract_agent_status("{}"), None);
    }
}
