use std::{path::Path, rc::Rc};
use anyhow::{Result, anyhow};
use crate::{config::Config, entry::{Entry, EntryAction}};
use rustc_hash::FxHashMap;

// TODO invalidate when config changes, keep hash of the config or timestamp
/// Simple database implementation using `FxHashMap`, probably slower than a proper
/// database but it's a lot simpler
///
/// It is essentially a read-only database as there is no need for modifying it if its outdated only
/// recreating it from scratch
#[derive(Debug, PartialEq, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct EntryDB {
    /// Time of creation in seconds
    pub timestamp: u64,

    pub by_id: FxHashMap<String, Rc<Entry>>,
    pub by_tag: FxHashMap<String, Vec<Rc<Entry>>>,
    pub entries: Vec<Rc<Entry>>,
    pub actions: Vec<Rc<EntryAction>>
}

impl EntryDB {
    const MAX_SIZE: u64 = 100 * 1024; // 100kB
    const OLD_THRESHOLD: u64 = 60 * 60 * 60; // 1 hour

    pub fn is_valid(&self, config_mtime: u64) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};

        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        elapsed > (self.timestamp + Self::OLD_THRESHOLD)
            && self.timestamp > config_mtime
    }

    pub fn from_entries(entries: Vec<Entry>) -> Self {
        let mut cache = Self::default();

        // new timestamp
        cache.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        cache.entries = entries.into_iter().map(|x| Rc::new(x)).collect::<Vec<_>>();

        // sort by name by default
        cache.entries.sort_by(|a, b| a.name.cmp(&b.name));

        for entry in &cache.entries {
            cache.by_id.insert(entry.id.clone(), Rc::clone(entry));

            for tag in &entry.tags {
                if let Some(tagged_entries) = cache.by_tag.get_mut(tag) {
                    tagged_entries.push(Rc::clone(entry));
                } else {
                    cache.by_tag.insert(tag.clone(), vec![Rc::clone(entry)]);
                }
            }
        }

        cache
    }

    pub fn from_io<T: std::io::Read>(input: &mut T) -> Result<Self> {
        use std::io::Read;

        // limit the size just in case
        let mut input = input.by_ref().take(Self::MAX_SIZE);
        let mut buf: Vec<u8> = Vec::new();
        input.read_to_end(&mut buf)?;

        if buf.len() >= Self::MAX_SIZE as usize {
            return Err(anyhow!("EntryDB binary size exceeded the maximum of {} bytes", Self::MAX_SIZE));
        }

        Ok(rkyv::from_bytes::<EntryDB, rkyv::rancor::Error>(&buf)?)
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let mut file = std::fs::OpenOptions::new().read(true).open(path)?;
        Self::from_io(&mut file)
    }

    pub fn save_io<T: std::io::Write>(&self, output: &mut T) -> Result<()> {
        let serialized = rkyv::to_bytes::<rkyv::rancor::Error>(self).unwrap();
        if serialized.len() >= Self::MAX_SIZE as usize {
            return Err(anyhow!("EntryDB binary size would exceed the maximum ({})", serialized.len()));
        }

        output.write_all(&serialized)?;

        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;

        self.save_io(&mut file)?;

        Ok(())
    }
}

/// Gets the cached entries if they are recent enough otherwise find them
pub fn get_cache(cache_path: Option<&Path>, config: &Config) -> Result<EntryDB> {
    if let Some(path) = cache_path {
        if let Ok(db) = EntryDB::from_file(path) && !db.is_valid(config.mtime) {
            return Ok(db);
        } else {
            let entries = crate::find_entries(config)?;
            let cache = EntryDB::from_entries(entries);

            cache.save(path)?;

            Ok(cache)
        }
    } else {
        Ok(EntryDB::from_entries(crate::find_entries(config)?))
    }
}



#[cfg(test)]
mod tests {
    use std::{io::Cursor, rc::Rc};
    use rustc_hash::FxHashSet;
    use crate::{entry::{Entry, EntryAction, EntryType}, entry_cache::EntryDB};

    #[test]
    fn entrydb_file() {
        let buf: Vec<u8> = Vec::with_capacity(EntryDB::MAX_SIZE as usize);
        let mut buf = Cursor::new(buf);

        let cache = EntryDB::from_entries(vec![
            Entry {
                id: "com.bravesoftware.brave".to_string(),
                entry_type: EntryType::Application,
                name: "Brave".to_string(),
                tags: { // TODO this is awfully ugly
                    let mut set: FxHashSet<String> = FxHashSet::default();
                    set.insert("banana".to_string());
                    set.insert("fruit".to_string());
                    set
                },
                actions: vec![
                    Rc::new(EntryAction {
                        name: "Brave Action 1".to_string(),
                        exec: "brave --something".to_string(),
                        icon: Some("brave-browser".to_string()),
                    }),
                    Rc::new(EntryAction {
                        name: "Brave Action 2".to_string(),
                        exec: "brave --yeeeah".to_string(),
                        ..Default::default()
                    }),
                ],
                ..Default::default()
            },
            Entry {
                id: "com.vivaldi.vivaldi".to_string(),
                entry_type: EntryType::Application,
                name: "Vivaldi".to_string(),
                tags: {
                    let mut set: FxHashSet<String> = FxHashSet::default();
                    set.insert("strawberry".to_string());
                    set.insert("fruit".to_string());
                    set
                },
                ..Default::default()
            },
        ]);

        cache.save_io(&mut buf).unwrap();
        assert!(buf.position() > 0, "serialized data is 0 bytes");

        // go back to beginning of the buffer
        buf.set_position(0);

        let deserialized = EntryDB::from_io(&mut buf).unwrap();
        assert_eq!(deserialized, cache);
    }
}
