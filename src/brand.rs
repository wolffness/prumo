//! Runtime program name for user-facing text.
//!
//! The fork ships the same binary under two names (`tuxedo` and `prumo`);
//! everything the user sees follows the name the binary was invoked as, while
//! config paths, cache paths, and file formats keep the upstream `tuxedo`
//! name so upstream merges stay cheap.

use std::path::Path;
use std::sync::OnceLock;

/// Names the program ships under. Anything else (e.g. a test-harness binary
/// like `title-1a2b3c`) falls back to the crate name so tests and snapshots
/// stay deterministic.
const KNOWN_NAMES: [&str; 2] = ["tuxedo", "prumo"];

/// The user-visible program name: the basename of the invoked binary when it
/// is a shipped name, otherwise the crate name.
pub fn app_name() -> &'static str {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        std::env::args_os()
            .next()
            .and_then(|arg0| {
                Path::new(&arg0)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .filter(|n| KNOWN_NAMES.contains(&n.as_str()))
            .unwrap_or_else(|| env!("CARGO_PKG_NAME").to_string())
    })
}
