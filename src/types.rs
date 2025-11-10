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
