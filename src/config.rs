use std::path::Path;
use anyhow::{Context, Result, anyhow};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

// TODO store the exec scripts in the config? its more readable
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Modification time of the config
    #[serde(skip)]
    pub mtime: u64,

    // tags for each id
    pub tags: FxHashMap<String, FxHashSet<String>>,
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

        let mut config: Self = serde_json::from_str(&contents)
            .with_context(|| anyhow!("could not parse config at {path:?}"))?;

        // save the modification time
        config.mtime = std::fs::metadata(path)?
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        Ok(config)
    }

    /// Save cache to path atomically
    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<()> {
        // its user readable config so write it pretty
        let contents = serde_json::to_string_pretty(self)
            .with_context(|| anyhow!("failed to serialize {self:?}"))?;

        let tmp_file = path.with_added_extension("tmp");
        std::fs::write(&tmp_file, &contents)
            .with_context(|| anyhow!("could not write to tmp file {tmp_file:?}"))?;

        std::fs::rename(&tmp_file, &path)
            .with_context(|| anyhow!("could not rename {tmp_file:?} to {path:?}"))?;

        Ok(())
    }
}
