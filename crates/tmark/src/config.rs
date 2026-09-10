//! `tmark.toml`: the workspace configuration the CLI and the language
//! server share. Design: `08-lsp.md` §Workspace, `05-diagnostics.md`
//! §Configuration.
//!
//! ```toml
//! profile = "canonical"          # canonical | strict | mkdocs
//! bibliography = ["refs.bib"]    # relative to this file
//!
//! [lint]
//! position-word = "off"          # off | hint | info | warning | error
//!
//! [press]
//! schema = "press-schema.json"   # merged into front-matter completion
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use tmark_fmt::Profile;
use tmark_ir::Code;
use tmark_lint::Config as LintConfig;
use tmark_registry::ResolveOptions;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Raw {
    profile: Option<String>,
    #[serde(default)]
    bibliography: Vec<PathBuf>,
    #[serde(default)]
    lint: BTreeMap<String, String>,
    #[serde(default)]
    press: PressRaw,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct PressRaw {
    schema: Option<PathBuf>,
}

/// A parsed `tmark.toml`. Paths are relative to `dir`, the directory of
/// the file (the current directory for a `Config` built in memory).
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Directory the relative paths of the file resolve against.
    pub dir: PathBuf,
    pub profile: Profile,
    /// `.bib` files, as written.
    pub bibliography: Vec<PathBuf>,
    pub lint: LintConfig,
    /// TeXSmith's `press` JSON schema for front-matter completion.
    pub press_schema: Option<PathBuf>,
}

impl Config {
    /// Parse the text of a `tmark.toml` that lives in `dir`.
    pub fn parse(text: &str, dir: &Path) -> Result<Config, String> {
        let raw: Raw = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let profile = match raw.profile.as_deref() {
            None | Some("canonical") => Profile::Canonical,
            Some("strict") => Profile::Strict,
            Some("mkdocs") => Profile::Mkdocs,
            Some(other) => {
                return Err(format!(
                    "unknown profile `{other}` (canonical, strict, mkdocs)"
                ))
            }
        };
        let mut lint = LintConfig::default();
        for (code, level) in &raw.lint {
            let Some(code) = Code::from_id(code) else {
                return Err(format!("unknown diagnostic code `{code}` under [lint]"));
            };
            lint.set(code, level)?;
        }
        Ok(Config {
            dir: dir.to_path_buf(),
            profile,
            bibliography: raw.bibliography,
            lint,
            press_schema: raw.press.schema,
        })
    }

    /// The nearest `tmark.toml` in `start` or a directory above it, read
    /// and parsed. `start` may be a file or a directory. `None` when there
    /// is none; `Some(Err)` when there is one that does not parse.
    #[cfg(feature = "fs")]
    pub fn discover(start: &Path) -> Option<Result<Config, String>> {
        let first = if start.is_dir() {
            Some(start)
        } else {
            start.parent()
        };
        let mut dir = first?;
        loop {
            let candidate = dir.join("tmark.toml");
            if candidate.is_file() {
                return Some(
                    std::fs::read_to_string(&candidate)
                        .map_err(|e| e.to_string())
                        .and_then(|text| Config::parse(&text, dir))
                        .map_err(|e| format!("{}: {e}", candidate.display())),
                );
            }
            dir = dir.parent()?;
        }
    }

    /// Resolution inputs for the document at `path`: the bibliography files
    /// expressed relative to the document, as `ResolveOptions` expects.
    pub fn resolve_options(&self, path: &Path) -> ResolveOptions {
        let base = path.parent().unwrap_or(Path::new(""));
        ResolveOptions {
            path: path.to_path_buf(),
            bibliography: self
                .bibliography
                .iter()
                .map(|bib| relative_to(&self.dir.join(bib), base))
                .collect(),
            ..Default::default()
        }
    }
}

/// `target` expressed relative to `base` (both relative or both absolute,
/// lexically), or `target` itself when they do not share a form.
fn relative_to(target: &Path, base: &Path) -> PathBuf {
    if target.is_absolute() != base.is_absolute() {
        return target.to_path_buf();
    }
    let t: Vec<_> = target.components().collect();
    let b: Vec<_> = base.components().collect();
    let common = t.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tmark_ir::Severity;

    #[test]
    fn parses_every_key() {
        let text = r#"
profile = "strict"
bibliography = ["refs.bib", "../shared/more.bib"]

[lint]
position-word = "off"
heading-skip = "error"

[press]
schema = "schema.json"
"#;
        let config = Config::parse(text, Path::new("/ws")).unwrap();
        assert_eq!(config.profile, Profile::Strict);
        assert_eq!(config.press_schema, Some(PathBuf::from("schema.json")));
        assert_eq!(
            config.lint.levels.get(&Code::HeadingSkip),
            Some(&Some(Severity::Error))
        );
        assert_eq!(config.lint.levels.get(&Code::PositionWord), Some(&None));
        let options = config.resolve_options(Path::new("/ws/docs/ch1.md"));
        assert_eq!(
            options.bibliography,
            [
                PathBuf::from("../refs.bib"),
                PathBuf::from("../../shared/more.bib")
            ]
        );
    }

    #[test]
    fn empty_file_is_the_default() {
        let config = Config::parse("", Path::new(".")).unwrap();
        assert_eq!(config.profile, Profile::Canonical);
        assert!(config.bibliography.is_empty());
    }

    #[test]
    fn rejects_unknown_keys_codes_and_levels() {
        assert!(Config::parse("colour = 1", Path::new(".")).is_err());
        assert!(Config::parse("[lint]\nnope = \"off\"", Path::new("."))
            .unwrap_err()
            .contains("nope"));
        assert!(Config::parse("[lint]\nheading-skip = \"loud\"", Path::new(".")).is_err());
        assert!(Config::parse("profile = \"pandoc\"", Path::new(".")).is_err());
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_to(Path::new("ws/refs.bib"), Path::new("ws/docs")),
            PathBuf::from("../refs.bib")
        );
        assert_eq!(
            relative_to(Path::new("ws/docs/a.bib"), Path::new("ws/docs")),
            PathBuf::from("a.bib")
        );
        assert_eq!(
            relative_to(Path::new("/abs/refs.bib"), Path::new("rel")),
            PathBuf::from("/abs/refs.bib")
        );
    }
}
