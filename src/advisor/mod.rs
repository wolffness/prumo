//! Optional AI advisor — an opt-in module that prioritizes / organizes the
//! user's own todo.txt via a configurable LLM backend.
//!
//! Design constraints (see the fork docs): the advisor is **off by default**
//! and fully decoupled — the rest of Prumo works with zero LLM dependency.
//! It never mutates the todo file; it prints a suggestion for the user to
//! review and apply by hand. The API key (Claude) comes from an environment
//! variable, never the config file.

pub mod github;
pub mod kanban;

use std::process::Command;

use anyhow::{Result, anyhow};

use crate::todo::Task;

/// Which LLM backend the advisor talks to. Ollama (local) is the default so
/// the advisor stays offline unless the user opts into a cloud backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Ollama,
    Claude,
}

impl Backend {
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ollama" => Some(Self::Ollama),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }

    /// Model used when the user hasn't set `advisor_model`.
    fn default_model(self) -> &'static str {
        match self {
            Self::Ollama => "llama3.2",
            // Per the Anthropic guidance, default to the flagship Opus model
            // unless the user names another via advisor_model.
            Self::Claude => "claude-opus-4-8",
        }
    }
}

/// Resolved advisor settings, derived from [`crate::config::Config`].
#[derive(Debug, Clone)]
pub struct AdvisorConfig {
    pub backend: Backend,
    /// Effective model — the configured value, or the backend default.
    pub model: String,
}

impl AdvisorConfig {
    /// Build from the raw config fields. Falls back to Ollama when the backend
    /// string is missing or unrecognized. O gating (quais projetos têm o
    /// advisor ligado) é decidido pelo chamador, não aqui.
    pub fn resolve(backend: Option<&str>, model: Option<&str>) -> Self {
        let backend = backend.and_then(Backend::parse).unwrap_or(Backend::Ollama);
        let model = model
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| backend.default_model().to_string());
        Self { backend, model }
    }
}

/// The advisor task the user requested on the CLI.
#[derive(Debug, Clone, Copy)]
pub enum Task_ {
    /// Suggest the top-N tasks to do next and why.
    Prioritize,
}

/// Linhas de tarefas locais abertas (raw), opcionalmente filtradas a um
/// `+project`. É a base da priorização, à qual o chamador anexa linhas do
/// GitHub antes de chamar [`advise`].
pub fn local_lines(tasks: &[Task], project: Option<&str>) -> Vec<String> {
    tasks
        .iter()
        .filter(|t| !t.done)
        .filter(|t| match project {
            Some(p) => t.projects.iter().any(|proj| proj == p),
            None => true,
        })
        .map(|t| t.raw.clone())
        .collect()
}

/// Run an advisor request over `lines` (tarefas locais + itens do GitHub já
/// montados), returning the model's suggestion as plain text. The caller
/// prints it; nothing is written to disk.
pub fn advise(cfg: &AdvisorConfig, kind: Task_, lines: &[String]) -> Result<String> {
    if lines.is_empty() {
        return Ok("Nenhuma tarefa aberta para priorizar.".to_string());
    }
    let prompt = build_prompt(kind, lines);
    match cfg.backend {
        Backend::Ollama => call_ollama(&cfg.model, &prompt),
        Backend::Claude => call_claude(&cfg.model, &prompt),
    }
}

/// Ranqueia as issues rumo a um `goal`, devolvendo `(número, tier 1-3, porquê)`
/// por issue reconhecida na resposta. Read-only; a IA só sugere. Cada item de
/// `issues` é `(número, título)`.
pub fn rank_issues(
    cfg: &AdvisorConfig,
    goal: &str,
    issues: &[(u64, String)],
) -> Result<Vec<(u64, u8, String)>> {
    if issues.is_empty() {
        return Ok(Vec::new());
    }
    let prompt = build_rank_prompt(goal, issues);
    let text = match cfg.backend {
        Backend::Ollama => call_ollama(&cfg.model, &prompt)?,
        Backend::Claude => call_claude(&cfg.model, &prompt)?,
    };
    Ok(parse_ranking(&text))
}

fn build_rank_prompt(goal: &str, issues: &[(u64, String)]) -> String {
    let list = issues
        .iter()
        .map(|(n, t)| format!("#{n} {t}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Você prioriza issues de GitHub para uma pessoa com TDAH/TEA que trabalha \
         sozinha. OBJETIVO: {goal}\n\nISSUES ABERTAS:\n{list}\n\n\
         Para CADA issue, avalie o quanto ela avança o OBJETIVO e devolva UMA linha \
         no formato EXATO:\n#<número> | <tier> | <porquê em até 12 palavras>\n\
         onde <tier> é 3 (essencial ao objetivo), 2 (ajuda) ou 1 (pouco relacionado). \
         Não escreva mais nada além dessas linhas. Responda em português."
    )
}

/// Parseia linhas `#<n> | <tier> | <porquê>` da resposta do modelo. Tolera
/// marcadores de lista e linhas fora do formato (ignoradas).
pub fn parse_ranking(text: &str) -> Vec<(u64, u8, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim().trim_start_matches(['-', '*', ' ']);
            let rest = line.strip_prefix('#')?;
            let mut parts = rest.splitn(3, '|');
            let number = parts.next()?.trim().parse::<u64>().ok()?;
            let tier = parts
                .next()?
                .trim()
                .parse::<u8>()
                .ok()
                .filter(|t| (1..=3).contains(t))?;
            let why = parts.next().unwrap_or("").trim().to_string();
            Some((number, tier, why))
        })
        .collect()
}

fn build_prompt(kind: Task_, lines: &[String]) -> String {
    let list = lines
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n");
    match kind {
        Task_::Prioritize => format!(
            "Você é um assistente de produtividade para uma pessoa com TDAH/TEA \
             que trabalha sozinha. Abaixo está a lista de tarefas todo.txt em aberto. \
             Itens marcados com `(?)` e um token `gh:owner/repo#N` são issues abertas \
             do GitHub que ainda NÃO estão no todo.txt — considere-as na priorização.\n\n\
             {list}\n\n\
             Escolha as 3 tarefas mais importantes para fazer AGORA, em ordem, \
             equilibrando urgência (datas `due:`) e esforço. Para cada uma, uma \
             linha curta com o porquê. Seja objetivo e não invente tarefas que \
             não estejam na lista. Responda em português."
        ),
    }
}

// ---------------------------------------------------------------------------
// Backends — shelled out via curl, matching the update-check pattern. Rust
// has no official Anthropic SDK, so raw HTTP is the idiomatic choice here.
// ---------------------------------------------------------------------------

const CURL_TIMEOUT_SECS: u64 = 60;

fn call_ollama(model: &str, prompt: &str) -> Result<String> {
    let body = serde_json_object(&[
        ("model", JsonVal::Str(model)),
        ("prompt", JsonVal::Str(prompt)),
        ("stream", JsonVal::Bool(false)),
    ]);
    let out = curl_post("http://localhost:11434/api/generate", &[], &body)
        .map_err(|e| anyhow!("Ollama request failed: {e}. Is `ollama serve` running?"))?;
    // Ollama returns {"response": "...", ...}.
    extract_json_string(&out, "response")
        .ok_or_else(|| anyhow!("could not parse Ollama response: {out}"))
}

fn call_claude(model: &str, prompt: &str) -> Result<String> {
    let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        anyhow!(
            "ANTHROPIC_API_KEY is not set. Export it in your shell to use the \
             Claude backend (the key is never read from config.toml)."
        )
    })?;
    let body = serde_json_object(&[
        ("model", JsonVal::Str(model)),
        ("max_tokens", JsonVal::Num(1024)),
        (
            "messages",
            JsonVal::Raw(&serde_json_messages("user", prompt)),
        ),
    ]);
    let headers = [
        format!("x-api-key: {key}"),
        "anthropic-version: 2023-06-01".to_string(),
        "content-type: application/json".to_string(),
    ];
    let header_refs: Vec<&str> = headers.iter().map(String::as_str).collect();
    let out = curl_post("https://api.anthropic.com/v1/messages", &header_refs, &body)
        .map_err(|e| anyhow!("Claude request failed: {e}"))?;
    // Response: {"content": [{"type":"text","text":"..."}], ...}.
    extract_claude_text(&out).ok_or_else(|| anyhow!("could not parse Claude response: {out}"))
}

fn curl_post(url: &str, headers: &[&str], body: &str) -> Result<String, String> {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "-m", &CURL_TIMEOUT_SECS.to_string(), "-X", "POST"]);
    for h in headers {
        cmd.args(["-H", h]);
    }
    if headers.is_empty() {
        cmd.args(["-H", "content-type: application/json"]);
    }
    cmd.args(["-d", body, url]);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("curl exited with {}: {}", out.status, err.trim()));
    }
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Minimal JSON helpers — building request bodies and pulling one field out of
// a well-formed response. Avoids a serde dependency for a couple of shapes;
// the parser mirrors the targeted scan already used in update.rs.
// ---------------------------------------------------------------------------

enum JsonVal<'a> {
    Str(&'a str),
    Num(i64),
    Bool(bool),
    /// Pre-serialized JSON, spliced in verbatim.
    Raw(&'a str),
}

fn serde_json_object(fields: &[(&str, JsonVal)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!("\"{k}\":"));
        match v {
            JsonVal::Str(s) => out.push_str(&json_quote(s)),
            JsonVal::Num(n) => out.push_str(&n.to_string()),
            JsonVal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            JsonVal::Raw(r) => out.push_str(r),
        }
    }
    out.push('}');
    out
}

fn serde_json_messages(role: &str, content: &str) -> String {
    format!(
        "[{{\"role\":\"{role}\",\"content\":{}}}]",
        json_quote(content)
    )
}

/// Escape a string as a JSON string literal (with surrounding quotes).
fn json_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Pull `"key":"<value>"` out of a flat JSON object, unescaping the value.
/// Exposed for tests.
pub fn extract_json_string(body: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let i = body.find(&needle)?;
    let rest = &body[i + needle.len()..];
    let colon = rest.find(':')?;
    let after = &rest[colon + 1..];
    let q = after.find('"')?;
    read_json_string_from(&after[q..])
}

/// The Claude response nests the text under `content[0].text`. Scan for the
/// first `"text"` field after a `"content"` key.
pub fn extract_claude_text(body: &str) -> Option<String> {
    let content_at = body.find("\"content\"")?;
    let after_content = &body[content_at..];
    extract_json_string(after_content, "text")
}

/// Read a JSON string literal starting at the opening quote, unescaping the
/// common escapes. Returns the decoded contents (no surrounding quotes).
fn read_json_string_from(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = s[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'b' => out.push('\u{0008}'),
                'f' => out.push('\u{000C}'),
                'u' => {
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_lines_filters_done_and_project() {
        use crate::todo::parse_line;
        let tasks: Vec<Task> = [
            "(A) Escrever plano +prumo",
            "x Concluída +prumo",
            "Comprar pão +casa",
        ]
        .iter()
        .filter_map(|l| parse_line(l).ok())
        .collect();

        // Sem filtro: só as abertas (2).
        assert_eq!(local_lines(&tasks, None).len(), 2);
        // Filtrado a +prumo: só a aberta desse projeto.
        assert_eq!(
            local_lines(&tasks, Some("prumo")),
            vec!["(A) Escrever plano +prumo".to_string()]
        );
    }

    #[test]
    fn backend_parse_and_defaults() {
        assert_eq!(Backend::parse("ollama"), Some(Backend::Ollama));
        assert_eq!(Backend::parse("Claude"), Some(Backend::Claude));
        assert_eq!(Backend::parse("gpt"), None);
        assert_eq!(Backend::Ollama.default_model(), "llama3.2");
        assert_eq!(Backend::Claude.default_model(), "claude-opus-4-8");
    }

    #[test]
    fn resolve_falls_back_to_ollama_and_default_model() {
        let c = AdvisorConfig::resolve(None, None);
        assert_eq!(c.backend, Backend::Ollama);
        assert_eq!(c.model, "llama3.2");

        let c = AdvisorConfig::resolve(Some("claude"), Some("claude-sonnet-5"));
        assert_eq!(c.backend, Backend::Claude);
        assert_eq!(c.model, "claude-sonnet-5");

        // Unknown backend → ollama; blank model → default.
        let c = AdvisorConfig::resolve(Some("xyz"), Some("  "));
        assert_eq!(c.backend, Backend::Ollama);
        assert_eq!(c.model, "llama3.2");
    }

    #[test]
    fn parse_ranking_reads_tiers_and_skips_noise() {
        let text = "Aqui vai:\n#12 | 3 | destrava o dashboard\n- #7 | 2 | ajuda na perf\n\
                    lixo aleatório\n#9 | 5 | fora do range\n#4 | 1 |\n";
        let r = parse_ranking(text);
        assert_eq!(
            r,
            vec![
                (12, 3, "destrava o dashboard".to_string()),
                (7, 2, "ajuda na perf".to_string()),
                (4, 1, String::new()),
            ]
        );
    }

    #[test]
    fn json_quote_escapes() {
        assert_eq!(json_quote("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }

    #[test]
    fn extract_ollama_response() {
        let body = r#"{"model":"llama3.2","response":"faça a tarefa 1","done":true}"#;
        assert_eq!(
            extract_json_string(body, "response").as_deref(),
            Some("faça a tarefa 1")
        );
    }

    #[test]
    fn extract_claude_text_from_content() {
        let body = r#"{"id":"msg_1","content":[{"type":"text","text":"1. Pagar aluguel"}],"model":"claude-opus-4-8"}"#;
        assert_eq!(
            extract_claude_text(body).as_deref(),
            Some("1. Pagar aluguel")
        );
    }

    #[test]
    fn read_string_unescapes_unicode() {
        assert_eq!(read_json_string_from(r#""café""#).as_deref(), Some("café"));
    }
}
