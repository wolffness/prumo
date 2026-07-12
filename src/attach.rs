//! Task file attachments. A task references attachments through `at:<name>`
//! tokens; the files themselves live in an `assets/` directory next to the
//! todo file, so a project folder stays self-contained and portable.
//!
//! The attach prompt (`t`) accepts a path typed by hand or dropped onto the
//! terminal (terminals paste the dragged file's path, usually with
//! shell-escaped spaces), moves the file into `assets/`, and appends the
//! token. Enter on a task opens its attachments with the system opener.

use std::path::{Path, PathBuf};

pub const ASSETS_SUBDIR: &str = "assets";

/// Directory attachments are moved into: `assets/` next to the todo file.
pub fn assets_dir(todo_path: &Path) -> PathBuf {
    todo_path
        .parent()
        .map_or_else(|| PathBuf::from(ASSETS_SUBDIR), |p| p.join(ASSETS_SUBDIR))
}

/// All `at:` tokens in a raw task line.
pub fn attach_rels_from_raw(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .filter_map(|token| token.strip_prefix("at:"))
        .map(|s| s.trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Resolve an `at:` token to a filesystem path. Absolute tokens are used
/// as-is; relative ones live under `assets_dir`.
pub fn path_for_rel(assets_dir: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        assets_dir.join(rel)
    }
}

/// Normalize a path as typed or dropped onto the terminal: trims whitespace,
/// strips surrounding quotes, undoes shell escapes (`\ ` etc.), and expands
/// a leading `~`.
pub fn clean_dropped_path(input: &str) -> PathBuf {
    let s = input.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(s);

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }

    if let Some(rest) = out.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(out)
}

/// Move `src` into `assets_dir`, returning the destination file name to use
/// in the `at:` token. The original is removed — the project folder becomes
/// the attachment's home, so cleaning up the Downloads copy can't break the
/// link. Whitespace in the name is dashed (todo.txt tokens are whitespace-
/// split) and name collisions get a `-1`, `-2`, … suffix instead of
/// overwriting an existing attachment.
pub fn move_into_assets(src: &Path, assets_dir: &Path) -> std::io::Result<String> {
    let file_name = src
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a file path"))?
        .to_string_lossy()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");

    std::fs::create_dir_all(assets_dir)?;
    let mut dest_name = file_name.clone();
    let mut n = 0usize;
    while assets_dir.join(&dest_name).exists() {
        n += 1;
        let p = Path::new(&file_name);
        let stem = p.file_stem().unwrap_or_default().to_string_lossy();
        let ext = p.extension().map(|e| e.to_string_lossy());
        dest_name = match &ext {
            Some(ext) => format!("{stem}-{n}.{ext}"),
            None => format!("{stem}-{n}"),
        };
    }
    let dest = assets_dir.join(&dest_name);
    // Prefer a true move; fall back to copy+remove when the source lives on
    // another volume (rename can't cross filesystems).
    if std::fs::rename(src, &dest).is_err() {
        std::fs::copy(src, &dest)?;
        std::fs::remove_file(src)?;
    }
    Ok(dest_name)
}

/// `file://` URI for a local path, percent-encoding everything outside the
/// RFC 3986 unreserved set (plus `/`). Terminals resolve these on click.
pub fn file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Open a file with the platform's default application, detached from the
/// TUI. Errors surface to the caller (which flashes them); a successful
/// spawn is fire-and-forget.
pub fn open_with_system(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]).arg(path);
        c
    };
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_percent_encodes_spaces_and_utf8() {
        assert_eq!(
            file_uri(Path::new("/tmp/a b/ç.pdf")),
            "file:///tmp/a%20b/%C3%A7.pdf"
        );
    }

    #[test]
    fn extracts_at_tokens() {
        assert_eq!(
            attach_rels_from_raw("Task at:spec.pdf body at:\"img.png\" note:x.md"),
            vec!["spec.pdf".to_string(), "img.png".to_string()]
        );
        assert!(attach_rels_from_raw("no attachments").is_empty());
    }

    #[test]
    fn cleans_finder_drop_escapes() {
        assert_eq!(
            clean_dropped_path("/Users/x/My\\ File.pdf "),
            PathBuf::from("/Users/x/My File.pdf")
        );
    }

    #[test]
    fn cleans_quoted_paths() {
        assert_eq!(
            clean_dropped_path("\"/tmp/a b.png\""),
            PathBuf::from("/tmp/a b.png")
        );
        assert_eq!(
            clean_dropped_path("'/tmp/c.pdf'"),
            PathBuf::from("/tmp/c.pdf")
        );
    }

    #[test]
    fn expands_tilde() {
        let home = std::env::var("HOME").expect("HOME set in tests");
        assert_eq!(
            clean_dropped_path("~/doc.pdf"),
            PathBuf::from(home).join("doc.pdf")
        );
    }

    #[test]
    fn assets_dir_is_sibling_of_todo_file() {
        assert_eq!(
            assets_dir(Path::new("/proj/todo.txt")),
            PathBuf::from("/proj/assets")
        );
    }

    fn temp_assets() -> (PathBuf, PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("tuxedo-attach-{}-{}", std::process::id(), n));
        let src_dir = base.join("src");
        let assets = base.join("assets");
        std::fs::create_dir_all(&src_dir).expect("mkdir");
        (src_dir, assets)
    }

    #[test]
    fn moves_file_and_dashes_spaces() {
        let (src_dir, assets) = temp_assets();
        let src = src_dir.join("client brief.pdf");
        std::fs::write(&src, b"pdf").expect("write src");
        let name = move_into_assets(&src, &assets).expect("move");
        assert_eq!(name, "client-brief.pdf");
        assert_eq!(std::fs::read(assets.join(&name)).expect("dest"), b"pdf");
        assert!(!src.exists(), "attach moves, not copies");
    }

    #[test]
    fn collision_appends_numeric_suffix() {
        let (src_dir, assets) = temp_assets();
        let src = src_dir.join("a.png");
        std::fs::write(&src, b"1").expect("write src");
        assert_eq!(move_into_assets(&src, &assets).expect("copy"), "a.png");
        std::fs::write(&src, b"2").expect("rewrite src");
        assert_eq!(move_into_assets(&src, &assets).expect("copy"), "a-1.png");
        assert_eq!(std::fs::read(assets.join("a.png")).expect("dest"), b"1");
        assert_eq!(std::fs::read(assets.join("a-1.png")).expect("dest"), b"2");
    }
}
