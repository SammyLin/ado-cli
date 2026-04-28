use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{env, fs};

pub const CONFIG_FILE: &str = ".ado.toml";

#[derive(Debug, Clone)]
pub struct Config {
    pub org: String,
    pub project: String,
    pub team: String,
    pub pat: String,
}

#[derive(Deserialize)]
struct TomlConfig {
    org: Option<String>,
    project: Option<String>,
    team: Option<String>,
    pat: Option<String>,
}

impl Config {
    /// Load config: .ado.toml (walk up dirs) → env vars as fallback per field.
    pub fn load() -> Result<Self> {
        let toml_cfg = find_config_file().and_then(|p| load_toml(&p).ok());

        let get = |toml_val: Option<&str>, env_key: &str| -> Result<String> {
            if let Some(v) = toml_val.filter(|s| !s.trim().is_empty()) {
                return Ok(v.to_string());
            }
            // Legacy .env fallback
            let _ = dotenvy::dotenv();
            env::var(env_key)
                .with_context(|| format!("missing {env_key} (set in {CONFIG_FILE} or env)"))
                .and_then(|v| {
                    if v.trim().is_empty() {
                        Err(anyhow!("{env_key} is empty"))
                    } else {
                        Ok(v)
                    }
                })
        };

        let tc = toml_cfg.as_ref();
        Ok(Self {
            org: get(tc.and_then(|c| c.org.as_deref()), "ADO_ORG")?,
            project: get(tc.and_then(|c| c.project.as_deref()), "ADO_PROJECT")?,
            team: get(tc.and_then(|c| c.team.as_deref()), "ADO_TEAM")?,
            pat: get(tc.and_then(|c| c.pat.as_deref()), "ADO_PAT")?,
        })
    }
}

/// Walk from cwd upward looking for .ado.toml.
pub fn find_config_file() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        let candidate = dir.join(CONFIG_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn load_toml(path: &Path) -> Result<TomlConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&content)
        .with_context(|| format!("parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn load_toml_parses_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        fs::write(&path, "org = \"O\"\nproject = \"P\"\nteam = \"T\"\npat = \"secret\"\n").unwrap();
        let cfg = load_toml(&path).unwrap();
        assert_eq!(cfg.org.as_deref(), Some("O"));
        assert_eq!(cfg.pat.as_deref(), Some("secret"));
    }

    #[test]
    fn load_toml_partial() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        fs::write(&path, "org = \"O\"\n").unwrap();
        let cfg = load_toml(&path).unwrap();
        assert_eq!(cfg.org.as_deref(), Some("O"));
        assert!(cfg.pat.is_none());
    }
}
