use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Site configuration from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    #[serde(default = "default_site_title")]
    pub title: String,
    #[serde(default = "default_site_url")]
    pub url: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default = "default_description")]
    pub description: String,
}

/// Theme configuration from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name")]
    pub name: String,
    #[serde(default)]
    pub custom_dir: Option<String>,
    #[serde(default)]
    pub variables: HashMap<String, serde_yaml::Value>,
}

/// Build configuration from config.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_content_dir")]
    pub content_dir: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
}

/// Complete config.yaml structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsgConfig {
    #[serde(default)]
    pub site: SiteConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub build: BuildConfig,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: default_site_title(),
            url: default_site_url(),
            author: default_author(),
            description: default_description(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            custom_dir: None,
            variables: HashMap::new(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            content_dir: default_content_dir(),
            output_dir: default_output_dir(),
            posts_per_page: default_posts_per_page(),
        }
    }
}

impl Default for SsgConfig {
    fn default() -> Self {
        Self {
            site: SiteConfig::default(),
            theme: ThemeConfig::default(),
            build: BuildConfig::default(),
        }
    }
}

fn default_site_title() -> String {
    "marshallku blog".to_string()
}

fn default_site_url() -> String {
    "https://marshallku.com".to_string()
}

fn default_author() -> String {
    "Marshall K".to_string()
}

fn default_description() -> String {
    "marshallku blog".to_string()
}

fn default_theme_name() -> String {
    "default".to_string()
}

fn default_content_dir() -> String {
    "content/posts".to_string()
}

fn default_output_dir() -> String {
    "dist".to_string()
}

fn default_posts_per_page() -> usize {
    10
}

pub fn load_config() -> Result<SsgConfig> {
    let config_path = Path::new("config.yaml");

    if !config_path.exists() {
        return Ok(SsgConfig::default());
    }

    let content = fs::read_to_string(config_path).context("Failed to read config.yaml")?;

    let config: SsgConfig =
        serde_yaml::from_str(&content).context("Failed to parse config.yaml")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SsgConfig::default();
        assert_eq!(config.site.title, "marshallku blog");
        assert_eq!(config.theme.name, "default");
        assert_eq!(config.build.posts_per_page, 10);
    }
}
