use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::slug;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct SearchIndex {
    pub version: String,
    pub posts: Vec<SearchEntry>,
}

#[derive(Debug, Serialize)]
pub struct SearchEntry {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub url: String,
    pub category: String,
    pub tags: Vec<String>,
    pub date: String,
}

pub struct SearchIndexGenerator {
    config: SsgConfig,
}

impl SearchIndexGenerator {
    pub fn new(config: SsgConfig) -> Self {
        Self { config }
    }

    pub fn generate(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n🔍 Generating search index...");

        let mut posts: Vec<SearchEntry> = metadata
            .posts
            .iter()
            .filter(|p| !p.frontmatter.draft)
            .map(|post| {
                let url = if self.config.build.encode_filenames {
                    format!(
                        "/{}/{}/",
                        slug::encode_for_url(&post.category),
                        slug::encode_for_url(&post.slug)
                    )
                } else {
                    format!("/{}/{}/", post.category, post.slug)
                };

                SearchEntry {
                    title: post.frontmatter.title.clone(),
                    description: post.frontmatter.description.clone(),
                    url,
                    category: post.category.clone(),
                    tags: post.frontmatter.tags.clone(),
                    date: post.frontmatter.date.posted.format("%Y-%m-%d").to_string(),
                }
            })
            .collect();

        posts.sort_by(|a, b| b.date.cmp(&a.date));

        let index = SearchIndex {
            version: "1.0".to_string(),
            posts,
        };

        let json = serde_json::to_string(&index)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("search-index.json");

        fs::write(&output_path, json)?;

        println!("   ✓ {} posts indexed", index.posts.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::MetadataCache;
    use crate::types::{Frontmatter, PostDate};
    use chrono::Utc;

    fn create_test_config() -> SsgConfig {
        SsgConfig::default()
    }

    fn create_test_metadata() -> MetadataCache {
        let mut metadata = MetadataCache::new();

        let frontmatter = Frontmatter {
            title: "Test Post".to_string(),
            date: PostDate::new(Utc::now()),
            tags: vec!["rust".to_string(), "test".to_string()],
            featured_image: None,
            description: Some("A test post".to_string()),
            draft: false,
        };

        metadata.upsert_post("test-post".to_string(), "dev".to_string(), frontmatter);

        metadata
    }

    #[test]
    fn test_search_entry_creation() {
        let config = create_test_config();
        let metadata = create_test_metadata();

        SearchIndexGenerator::new(config);

        let post = &metadata.posts[0];
        let entry = SearchEntry {
            title: post.frontmatter.title.clone(),
            description: post.frontmatter.description.clone(),
            url: format!("/{}/{}/", post.category, post.slug),
            category: post.category.clone(),
            tags: post.frontmatter.tags.clone(),
            date: post.frontmatter.date.posted.format("%Y-%m-%d").to_string(),
        };

        assert_eq!(entry.title, "Test Post");
        assert_eq!(entry.url, "/dev/test-post/");
        assert_eq!(entry.tags.len(), 2);
    }
}
