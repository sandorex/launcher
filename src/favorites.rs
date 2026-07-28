use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favorites(pub Vec<String>);

impl Favorites {
    /// Read the favorites from path
    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| anyhow!("could not read {path:?}"))?;

        serde_json::from_str(&contents)
            .with_context(|| anyhow!("could not parse json in {path:?}"))
    }

    /// Save cache to path atomically
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = serde_json::to_string(self)
            .with_context(|| anyhow!("failed to serialize {self:?}"))?;

        let tmp_file = path.with_added_extension("tmp");
        std::fs::write(&tmp_file, &contents)
            .with_context(|| anyhow!("could not write to tmp file {tmp_file:?}"))?;

        std::fs::rename(&tmp_file, &path)
            .with_context(|| anyhow!("could not rename {tmp_file:?} to {path:?}"))?;

        Ok(())
    }
}

impl From<Vec<String>> for Favorites {
    fn from(value: Vec<String>) -> Self {
        Self(value)
    }
}
