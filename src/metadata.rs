use crate::types::{Category, Frontmatter};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Lightweight post metadata for index generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetadata {
    pub slug: String,
    pub frontmatter: Frontmatter,
}

/// Metadata cache for efficient index generation
#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataCache {
    pub version: String,
    pub posts: Vec<PostMetadata>,
    pub categories: HashMap<String, usize>,
    pub tags: HashMap<String, usize>,
    #[serde(default)]
    pub category_info: Vec<Category>,
}

impl MetadataCache {
    /// Load metadata cache from disk
    pub fn load() -> Result<Self> {
        let cache_path = ".build-cache/metadata.json";

        if std::path::Path::new(cache_path).exists() {
            let content = fs::read_to_string(cache_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    /// Create a new empty metadata cache
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            posts: Vec::new(),
            categories: HashMap::new(),
            tags: HashMap::new(),
            category_info: Vec::new(),
        }
    }

    /// Set category information
    pub fn set_category_info(&mut self, categories: Vec<Category>) {
        self.category_info = categories;
    }

    /// Get category information
    pub fn get_category_info(&self) -> &[Category] {
        &self.category_info
    }

    /// Add or update a post in the metadata cache
    pub fn upsert_post(&mut self, slug: String, frontmatter: Frontmatter) {
        // Remove old entry if exists
        self.posts.retain(|p| p.slug != slug);

        // Add new entry
        self.posts.push(PostMetadata {
            slug,
            frontmatter,
        });

        // Recalculate category and tag counts
        self.recalculate_stats();
    }

    /// Recalculate category and tag statistics
    fn recalculate_stats(&mut self) {
        self.categories.clear();
        self.tags.clear();

        for post in &self.posts {
            // Count categories
            *self.categories.entry(post.frontmatter.category.clone()).or_insert(0) += 1;

            // Count tags
            for tag in &post.frontmatter.tags {
                *self.tags.entry(tag.clone()).or_insert(0) += 1;
            }
        }
    }

    /// Get posts by category
    pub fn get_posts_by_category(&self, category: &str) -> Vec<&PostMetadata> {
        self.posts
            .iter()
            .filter(|p| p.frontmatter.category == category)
            .collect()
    }

    /// Get posts by tag
    pub fn get_posts_by_tag(&self, tag: &str) -> Vec<&PostMetadata> {
        self.posts
            .iter()
            .filter(|p| p.frontmatter.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get recent posts (sorted by date, descending)
    pub fn get_recent_posts(&self, limit: usize) -> Vec<&PostMetadata> {
        let mut posts: Vec<_> = self.posts.iter().collect();
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
        posts.into_iter().take(limit).collect()
    }

    /// Get all unique categories
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<_> = self.categories.keys().cloned().collect();
        categories.sort();
        categories
    }

    /// Get all unique tags
    pub fn get_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> = self.tags.keys().cloned().collect();
        tags.sort();
        tags
    }

    /// Save metadata cache to disk
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(".build-cache")?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(".build-cache/metadata.json", json)?;
        Ok(())
    }
}

impl Default for MetadataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_frontmatter(category: &str, tags: Vec<&str>) -> Frontmatter {
        Frontmatter {
            title: "Test Post".to_string(),
            date: Utc::now(),
            category: category.to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            featured_image: None,
            description: None,
            draft: false,
        }
    }

    #[test]
    fn test_upsert_post() {
        let mut cache = MetadataCache::new();

        let fm = create_test_frontmatter("dev", vec!["rust", "webdev"]);
        cache.upsert_post("test-post".to_string(), fm);

        assert_eq!(cache.posts.len(), 1);
        assert_eq!(cache.categories.get("dev"), Some(&1));
        assert_eq!(cache.tags.get("rust"), Some(&1));
    }

    #[test]
    fn test_get_posts_by_category() {
        let mut cache = MetadataCache::new();

        cache.upsert_post("post1".to_string(), create_test_frontmatter("dev", vec![]));
        cache.upsert_post("post2".to_string(), create_test_frontmatter("chat", vec![]));
        cache.upsert_post("post3".to_string(), create_test_frontmatter("dev", vec![]));

        let dev_posts = cache.get_posts_by_category("dev");
        assert_eq!(dev_posts.len(), 2);
    }

    #[test]
    fn test_get_posts_by_tag() {
        let mut cache = MetadataCache::new();

        cache.upsert_post("post1".to_string(), create_test_frontmatter("dev", vec!["rust"]));
        cache.upsert_post("post2".to_string(), create_test_frontmatter("dev", vec!["rust", "webdev"]));
        cache.upsert_post("post3".to_string(), create_test_frontmatter("chat", vec!["webdev"]));

        let rust_posts = cache.get_posts_by_tag("rust");
        assert_eq!(rust_posts.len(), 2);
    }
}
