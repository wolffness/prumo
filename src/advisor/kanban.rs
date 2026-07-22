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

/// One board card: an issue plus its Project fields. `item_id` is the
/// ProjectV2Item node id, needed to mutate the item's fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KanbanCard {
    pub item_id: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub status: String,
    pub agent: String,
}

/// Board metadata needed for writes: field ids and their option ids,
/// fetched from the API on refresh (never hardcoded).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardMeta {
    pub status_field: String,
    pub agent_field: String,
    /// `(option_id, option_name)` in board order.
    pub status_options: Vec<(String, String)>,
    pub agent_options: Vec<(String, String)>,
}

/// GraphQL query flattened to `item_id\trepo\tnumber\ttitle\tstatus\tagent`
/// lines by gh's `--jq`. Draft items and PRs (no issue content) are skipped.
const QUERY: &str = "query($id:ID!){node(id:$id){... on ProjectV2{items(first:100){nodes{id content{... on Issue{number title repository{nameWithOwner}}} fieldValues(first:20){nodes{... on ProjectV2ItemFieldSingleSelectValue{name field{... on ProjectV2SingleSelectField{name}}}}}}}}}}";
const JQ: &str = r#".data.node.items.nodes[] | select(.content.number != null) | [.id, .content.repository.nameWithOwner, (.content.number|tostring), .content.title, (first(.fieldValues.nodes[]? | select(.field.name? == "Status") | .name) // ""), (first(.fieldValues.nodes[]? | select(.field.name? == "Agent") | .name) // "")] | @tsv"#;

/// Single-select fields of the board, flattened to
/// `field_id\tfield_name\toption_id\toption_name` lines.
const META_QUERY: &str = "query($id:ID!){node(id:$id){... on ProjectV2{fields(first:30){nodes{... on ProjectV2SingleSelectField{id name options{id name}}}}}}}";
const META_JQ: &str = r#".data.node.fields.nodes[] | select((.name? // "") == "Status" or (.name? // "") == "Agent") | .id as $f | .name as $n | .options[] | [$f, $n, .id, .name] | @tsv"#;

/// Fetches the board cards. First 100 items only — a personal board; revisit
/// with pagination if it ever grows past that.
pub fn fetch_board() -> Result<Vec<KanbanCard>> {
    let query_arg = format!("query={QUERY}");
    let id_arg = format!("id={}", project_id());
    let out = super::github::gh(&["api", "graphql", "-f", &query_arg, "-f", &id_arg, "--jq", JQ])?;
    Ok(parse_kanban_tsv(&out))
}

/// Parses `item_id\trepo\tnumber\ttitle\tstatus\tagent` (one card per line).
/// Malformed lines are skipped, like the other parsers in this module.
/// An empty status means the item was never placed on the board → "Todo".
pub fn parse_kanban_tsv(stdout: &str) -> Vec<KanbanCard> {
    stdout
        .lines()
        .filter_map(|l| {
            let mut parts = l.split('\t');
            let item_id = parts.next()?.trim();
            let repo = parts.next()?.trim();
            let number: u64 = parts.next()?.trim().parse().ok()?;
            let title = parts.next()?.trim();
            let status = parts.next().unwrap_or("").trim();
            let agent = parts.next().unwrap_or("").trim();
            if item_id.is_empty() || repo.is_empty() || title.is_empty() {
                return None;
            }
            Some(KanbanCard {
                item_id: item_id.to_string(),
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

/// Parses `field_id\tfield_name\toption_id\toption_name` into [`BoardMeta`].
pub fn parse_board_meta_tsv(stdout: &str) -> BoardMeta {
    let mut meta = BoardMeta::default();
    for l in stdout.lines() {
        let mut parts = l.split('\t');
        let (Some(field_id), Some(field_name), Some(opt_id), Some(opt_name)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let opt = (opt_id.trim().to_string(), opt_name.trim().to_string());
        match field_name.trim() {
            "Status" => {
                meta.status_field = field_id.trim().to_string();
                meta.status_options.push(opt);
            }
            "Agent" => {
                meta.agent_field = field_id.trim().to_string();
                meta.agent_options.push(opt);
            }
            _ => {}
        }
    }
    meta
}

/// Fetches the board's field/option ids (needed for writes).
pub fn fetch_board_meta() -> Result<BoardMeta> {
    let query_arg = format!("query={META_QUERY}");
    let id_arg = format!("id={}", project_id());
    let out = super::github::gh(&["api", "graphql", "-f", &query_arg, "-f", &id_arg, "--jq", META_JQ])?;
    Ok(parse_board_meta_tsv(&out))
}

/// Sets a single-select field of a board item (e.g. move Status, set Agent).
/// Blocking call; the caller only mutates local state on `Ok`.
pub fn set_item_field(item_id: &str, field_id: &str, option_id: &str) -> Result<()> {
    const MUTATION: &str = "mutation($p:ID!,$i:ID!,$f:ID!,$o:String!){updateProjectV2ItemFieldValue(input:{projectId:$p,itemId:$i,fieldId:$f,value:{singleSelectOptionId:$o}}){projectV2Item{id}}}";
    let q = format!("query={MUTATION}");
    let p = format!("p={}", project_id());
    let i = format!("i={item_id}");
    let f = format!("f={field_id}");
    let o = format!("o={option_id}");
    super::github::gh(&["api", "graphql", "-f", &q, "-f", &p, "-f", &i, "-f", &f, "-f", &o])?;
    Ok(())
}

/// Board id: env override or the built-in default.
fn project_id() -> String {
    std::env::var("PRUMO_KANBAN_PROJECT").unwrap_or_else(|_| DEFAULT_PROJECT_ID.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cards_and_skips_malformed() {
        let out = "PVTI_a\to/r\t1\tEpic title\tTodo\t\n\
                   PVTI_b\to/r\t2\tCard two\tIn Progress\topus\n\
                   not-a-card\n\
                   PVTI_c\to/r\tNaN\tbad number\tTodo\tx\n";
        let cards = parse_kanban_tsv(out);
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].number, 1);
        assert_eq!(cards[0].item_id, "PVTI_a");
        assert_eq!(cards[0].agent, "");
        assert_eq!(cards[1].status, "In Progress");
        assert_eq!(cards[1].agent, "opus");
    }

    #[test]
    fn empty_status_defaults_to_todo() {
        let cards = parse_kanban_tsv("PVTI_x\to/r\t7\tNo status yet\t\t\n");
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].status, "Todo");
    }

    #[test]
    fn parses_board_meta_by_field_name() {
        let out = "F1\tStatus\to1\tTodo\n\
                   F1\tStatus\to2\tIn Progress\n\
                   F1\tStatus\to3\tDone\n\
                   F2\tAgent\ta1\tsonnet\n\
                   F2\tAgent\ta2\topus\n\
                   garbage-line\n";
        let meta = parse_board_meta_tsv(out);
        assert_eq!(meta.status_field, "F1");
        assert_eq!(meta.agent_field, "F2");
        assert_eq!(meta.status_options.len(), 3);
        assert_eq!(meta.agent_options[1], ("a2".to_string(), "opus".to_string()));
    }
}
