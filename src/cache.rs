use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildCache {
    pub version: String,
    pub entries: HashMap<String, CacheEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: String,
    pub template_hash: String,
    pub output_path: String,
    pub built_at: String,
}

impl BuildCache {
    pub fn load() -> Result<Self> {
        let cache_path = Path::new(".build-cache/cache.json");

        if cache_path.exists() {
            let content = fs::read_to_string(cache_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            entries: HashMap::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(".build-cache")?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(".build-cache/cache.json", json)?;
        Ok(())
    }

    pub fn needs_rebuild(&self, path: &Path, current_hash: &str, current_template_hash: &str) -> bool {
        let path_str = path.to_string_lossy();

        match self.entries.get(path_str.as_ref()) {
            None => true,
            Some(entry) => {
                entry.file_hash != current_hash || entry.template_hash != current_template_hash
            }
        }
    }

    pub fn update_entry(
        &mut self,
        path: &Path,
        hash: String,
        template_hash: String,
        output: String,
    ) {
        let path_str = path.to_string_lossy().to_string();

        self.entries.insert(
            path_str,
            CacheEntry {
                file_hash: hash,
                template_hash,
                output_path: output,
                built_at: chrono::Utc::now().to_rfc3339(),
            },
        );
    }
}

impl Default for BuildCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn hash_file(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let hash = blake3::hash(&content);
    Ok(hash.to_hex().to_string())
}

pub fn hash_directory(dir: &Path) -> Result<String> {
    use walkdir::WalkDir;

    let mut hasher = blake3::Hasher::new();
    let mut files: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    files.sort_by_key(|e| e.path().to_path_buf());

    for entry in files {
        let path = entry.path();
        if let Ok(content) = fs::read(path) {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(&content);
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "Hello, world!").unwrap();

        let hash1 = hash_file(file.path()).unwrap();
        let hash2 = hash_file(file.path()).unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_cache_needs_rebuild() {
        let cache = BuildCache::new();
        let path = Path::new("test.md");

        assert!(cache.needs_rebuild(path, "abc123", "template_hash"));
    }

    #[test]
    fn test_cache_update_entry() {
        let mut cache = BuildCache::new();
        let path = Path::new("test.md");

        cache.update_entry(
            path,
            "abc123".to_string(),
            "def456".to_string(),
            "dist/test/index.html".to_string(),
        );

        assert!(!cache.needs_rebuild(path, "abc123", "def456"));
        assert!(cache.needs_rebuild(path, "different_hash", "def456"));
        assert!(cache.needs_rebuild(path, "abc123", "different_template_hash"));
    }
}
