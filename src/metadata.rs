use crate::types::{Category, Frontmatter};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMetadata {
    pub slug: String,
    pub category: String,
    pub frontmatter: Frontmatter,
}

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
    pub fn load() -> Result<Self> {
        let cache_path = ".build-cache/metadata.json";

        if std::path::Path::new(cache_path).exists() {
            let content = fs::read_to_string(cache_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            posts: Vec::new(),
            categories: HashMap::new(),
            tags: HashMap::new(),
            category_info: Vec::new(),
        }
    }

    pub fn set_category_info(&mut self, categories: Vec<Category>) {
        self.category_info = categories;
    }

    pub fn get_category_info(&self) -> &[Category] {
        &self.category_info
    }

    pub fn upsert_post(&mut self, slug: String, category: String, frontmatter: Frontmatter) {
        self.posts.retain(|p| p.slug != slug);

        self.posts.push(PostMetadata {
            slug,
            category,
            frontmatter,
        });

        self.recalculate_stats();
    }

    fn recalculate_stats(&mut self) {
        self.categories.clear();
        self.tags.clear();

        for post in &self.posts {
            *self.categories.entry(post.category.clone()).or_insert(0) += 1;

            for tag in &post.frontmatter.tags {
                *self.tags.entry(tag.clone()).or_insert(0) += 1;
            }
        }
    }

    pub fn get_posts_by_category(&self, category: &str) -> Vec<&PostMetadata> {
        self.posts
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    pub fn get_posts_by_tag(&self, tag: &str) -> Vec<&PostMetadata> {
        self.posts
            .iter()
            .filter(|p| p.frontmatter.tags.contains(&tag.to_string()))
            .collect()
    }

    pub fn get_recent_posts(&self, limit: usize) -> Vec<&PostMetadata> {
        let mut posts: Vec<_> = self.posts.iter().collect();
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));
        posts.into_iter().take(limit).collect()
    }

    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<_> = self.categories.keys().cloned().collect();
        categories.sort();
        categories
    }

    pub fn get_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> = self.tags.keys().cloned().collect();
        tags.sort();
        tags
    }

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

    fn create_test_post(category: &str, tags: Vec<&str>) -> (String, Frontmatter) {
        let frontmatter = Frontmatter {
            title: "Test Post".to_string(),
            date: crate::types::PostDate::new(Utc::now()),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            featured_image: None,
            description: None,
            draft: false,
        };
        (category.to_string(), frontmatter)
    }

    #[test]
    fn test_upsert_post() {
        let mut cache = MetadataCache::new();

        let (category, fm) = create_test_post("dev", vec!["rust", "webdev"]);
        cache.upsert_post("test-post".to_string(), category, fm);

        assert_eq!(cache.posts.len(), 1);
        assert_eq!(cache.categories.get("dev"), Some(&1));
        assert_eq!(cache.tags.get("rust"), Some(&1));
    }

    #[test]
    fn test_get_posts_by_category() {
        let mut cache = MetadataCache::new();

        let (cat1, fm1) = create_test_post("dev", vec![]);
        let (cat2, fm2) = create_test_post("chat", vec![]);
        let (cat3, fm3) = create_test_post("dev", vec![]);

        cache.upsert_post("post1".to_string(), cat1, fm1);
        cache.upsert_post("post2".to_string(), cat2, fm2);
        cache.upsert_post("post3".to_string(), cat3, fm3);

        let dev_posts = cache.get_posts_by_category("dev");
        assert_eq!(dev_posts.len(), 2);
    }

    #[test]
    fn test_get_posts_by_tag() {
        let mut cache = MetadataCache::new();

        let (cat1, fm1) = create_test_post("dev", vec!["rust"]);
        let (cat2, fm2) = create_test_post("dev", vec!["rust", "webdev"]);
        let (cat3, fm3) = create_test_post("chat", vec!["webdev"]);

        cache.upsert_post("post1".to_string(), cat1, fm1);
        cache.upsert_post("post2".to_string(), cat2, fm2);
        cache.upsert_post("post3".to_string(), cat3, fm3);

        let rust_posts = cache.get_posts_by_tag("rust");
        assert_eq!(rust_posts.len(), 2);
    }
}
