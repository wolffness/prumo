//! Read-only Kanban view of a GitHub Project v2 board.
//!
//! Fetches board items via `gh api graphql --jq` — the response is flattened
//! to TSV by gh's built-in jq, so this module stays dependency-free and the
//! parser mirrors the other TSV parsers in `advisor::github`. Write support
//! (moving Status, setting Agent) lands in a later slice.

use anyhow::Result;

/// Default Project v2 node id (the personal "command center" board).
/// Override with `PRUMO_KANBAN_PROJECT` to point at another board.
pub const DEFAULT_PROJECT_ID: &str = "PVT_kwHOAGXaTM4BeIxX";

/// Canonical board columns, in display order.
pub const COLUMNS: [&str; 3] = ["Todo", "In Progress", "Done"];

/// One board card: an issue plus its Project fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanCard {
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub status: String,
    pub agent: String,
}

/// GraphQL query flattened to `repo\tnumber\ttitle\tstatus\tagent` lines by
/// gh's `--jq`. Draft items and PRs (no issue content) are skipped.
const QUERY: &str = "query($id:ID!){node(id:$id){... on ProjectV2{items(first:100){nodes{content{... on Issue{number title repository{nameWithOwner}}} fieldValues(first:20){nodes{... on ProjectV2ItemFieldSingleSelectValue{name field{... on ProjectV2SingleSelectField{name}}}}}}}}}}";
const JQ: &str = r#".data.node.items.nodes[] | select(.content.number != null) | [.content.repository.nameWithOwner, (.content.number|tostring), .content.title, (first(.fieldValues.nodes[]? | select(.field.name? == "Status") | .name) // ""), (first(.fieldValues.nodes[]? | select(.field.name? == "Agent") | .name) // "")] | @tsv"#;

/// Fetches the board cards. First 100 items only — a personal board; revisit
/// with pagination if it ever grows past that.
pub fn fetch_board() -> Result<Vec<KanbanCard>> {
    let project = std::env::var("PRUMO_KANBAN_PROJECT")
        .unwrap_or_else(|_| DEFAULT_PROJECT_ID.to_string());
    let query_arg = format!("query={QUERY}");
    let id_arg = format!("id={project}");
    let out = super::github::gh(&["api", "graphql", "-f", &query_arg, "-f", &id_arg, "--jq", JQ])?;
    Ok(parse_kanban_tsv(&out))
}

/// Parses `repo\tnumber\ttitle\tstatus\tagent` (one card per line).
/// Malformed lines are skipped, like the other parsers in this module.
/// An empty status means the item was never placed on the board → "Todo".
pub fn parse_kanban_tsv(stdout: &str) -> Vec<KanbanCard> {
    stdout
        .lines()
        .filter_map(|l| {
            let mut parts = l.split('\t');
            let repo = parts.next()?.trim();
            let number: u64 = parts.next()?.trim().parse().ok()?;
            let title = parts.next()?.trim();
            let status = parts.next().unwrap_or("").trim();
            let agent = parts.next().unwrap_or("").trim();
            if repo.is_empty() || title.is_empty() {
                return None;
            }
            Some(KanbanCard {
                repo: repo.to_string(),
                number,
                title: title.to_string(),
                status: if status.is_empty() {
                    COLUMNS[0].to_string()
                } else {
                    status.to_string()
                },
                agent: agent.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cards_and_skips_malformed() {
        let out = "o/r\t1\tEpic title\tTodo\t\n\
                   o/r\t2\tCard two\tIn Progress\topus\n\
                   not-a-card\n\
                   o/r\tNaN\tbad number\tTodo\tx\n";
        let cards = parse_kanban_tsv(out);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].number, 1);
        assert_eq!(cards[0].agent, "");
        assert_eq!(cards[1].status, "In Progress");
        assert_eq!(cards[1].agent, "opus");
    }

    #[test]
    fn empty_status_defaults_to_todo() {
        let cards = parse_kanban_tsv("o/r\t7\tNo status yet\t\t\n");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].status, "Todo");
    }
}
