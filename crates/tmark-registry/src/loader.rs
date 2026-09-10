//! The one I/O seam of the core (design 06 §The Loader seam).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Returns the text of a path relative to another file, or `None`.
pub trait Loader {
    fn load(&self, from: &Path, rel: &str) -> Option<String>;
}

/// Resolve `rel` against the directory of `from` (a file path) or `from`
/// itself when it is a directory-like base.
pub fn join(from: &Path, rel: &str) -> PathBuf {
    let base = if from.extension().is_some() {
        from.parent().unwrap_or(Path::new(""))
    } else {
        from
    };
    let rel = Path::new(rel);
    if rel.is_absolute() {
        rel.to_path_buf()
    } else {
        normalise(&base.join(rel))
    }
}

/// Remove `.` and resolve `..` textually.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A map of paths to texts, for tests and for WASM.
#[derive(Debug, Default)]
pub struct MemoryLoader {
    files: BTreeMap<PathBuf, String>,
}

impl MemoryLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        self.files.insert(normalise(&path.into()), text.into());
        self
    }
}

impl Loader for MemoryLoader {
    fn load(&self, from: &Path, rel: &str) -> Option<String> {
        self.files.get(&join(from, rel)).cloned()
    }
}

/// The file system.
#[cfg(feature = "fs")]
#[derive(Debug, Default)]
pub struct FsLoader;

#[cfg(feature = "fs")]
impl Loader for FsLoader {
    fn load(&self, from: &Path, rel: &str) -> Option<String> {
        std::fs::read_to_string(join(from, rel)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_relative_to_the_file_directory() {
        assert_eq!(
            join(Path::new("docs/a.md"), "b.md"),
            PathBuf::from("docs/b.md")
        );
        assert_eq!(
            join(Path::new("docs/a.md"), "../x/y.md"),
            PathBuf::from("x/y.md")
        );
        assert_eq!(join(Path::new("docs"), "b.md"), PathBuf::from("docs/b.md"));
        let loader = MemoryLoader::new().with("docs/b.md", "hi");
        assert_eq!(
            loader.load(Path::new("docs/a.md"), "./b.md").as_deref(),
            Some("hi")
        );
        assert!(loader.load(Path::new("docs/a.md"), "c.md").is_none());
    }
}
