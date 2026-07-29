use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    // tags for each id
    pub tags: HashMap<String, Vec<String>>,
}

impl Config {
    /// Read the favorites from path or returns default if it does not exist
    pub fn read(path: &Path) -> Result<Self> {
        let contents = match std::fs::read_to_string(path) {
            Ok(x) => Ok(x),
            Err(err) => {
                use std::io::ErrorKind;
                match err.kind() {
                    // return default if the file does not exist
                    ErrorKind::NotFound => return Ok(Self::default()),

                    // propagate other errors
                    _ => Err(err),
                }
            },
        }.with_context(|| anyhow!("could not read config at {path:?}"))?;

        serde_json::from_str(&contents)
            .with_context(|| anyhow!("could not parse config at {path:?}"))
    }

    /// Save cache to path atomically
    #[allow(dead_code)]
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
