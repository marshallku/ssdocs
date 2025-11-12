use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub date: DateTime<Utc>,
    pub category: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Debug, Clone)]
pub struct Post {
    pub slug: String,
    pub frontmatter: Frontmatter,
    pub content: String,
    pub rendered_html: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub site_title: String,
    pub site_url: String,
    pub author: String,
    pub content_dir: String,
    pub template_dir: String,
    pub output_dir: String,
    pub posts_per_page: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site_title: "marshallku blog".to_string(),
            site_url: "https://marshallku.com".to_string(),
            author: "Marshall K".to_string(),
            content_dir: "content/posts".to_string(),
            template_dir: "templates".to_string(),
            output_dir: "dist".to_string(),
            posts_per_page: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    /// URL slug (same as directory name)
    #[serde(default)]
    pub slug: String,

    /// Display name (from .category.yaml or capitalized slug)
    #[serde(default)]
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: String,

    /// Sort order (lower = first)
    #[serde(default = "default_category_index")]
    pub index: i32,

    /// Hide from navigation
    #[serde(default)]
    pub hidden: bool,

    /// Optional icon identifier
    #[serde(default)]
    pub icon: Option<String>,

    /// Optional color hex code
    #[serde(default)]
    pub color: Option<String>,

    /// Optional cover image path
    #[serde(default)]
    pub cover_image: Option<String>,
}

fn default_category_index() -> i32 {
    999
}
