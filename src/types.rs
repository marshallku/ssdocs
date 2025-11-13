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
