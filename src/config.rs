use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub org: String,
    pub project: String,
    pub team: String,
    pub pat: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // dotenvy is best-effort: if no .env in cwd or parents, ignore.
        let _ = dotenvy::dotenv();

        let org = require("ADO_ORG")?;
        let project = require("ADO_PROJECT")?;
        let team = require("ADO_TEAM")?;
        let pat = require("ADO_PAT")?;

        Ok(Self {
            org,
            project,
            team,
            pat,
        })
    }
}

fn require(key: &str) -> Result<String> {
    env::var(key)
        .with_context(|| format!("missing env var {key} (set in shell or .env)"))
        .and_then(|v| {
            if v.trim().is_empty() {
                Err(anyhow::anyhow!("env var {key} is empty"))
            } else {
                Ok(v)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_missing_var() {
        env::remove_var("ADO_TEST_MISSING_XYZ");
        assert!(require("ADO_TEST_MISSING_XYZ").is_err());
    }

    #[test]
    fn require_empty_var() {
        env::set_var("ADO_TEST_EMPTY", "  ");
        assert!(require("ADO_TEST_EMPTY").is_err());
    }

    #[test]
    fn require_present_var() {
        env::set_var("ADO_TEST_OK", "value");
        assert_eq!(require("ADO_TEST_OK").unwrap(), "value");
    }
}
