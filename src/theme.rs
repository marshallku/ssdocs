use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::Tera;

use crate::config::SsgConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub variables: HashMap<String, serde_yaml::Value>,
    #[serde(default)]
    pub hooks: Vec<ThemeHook>,
    #[serde(default)]
    pub required_templates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeHook {
    pub name: String,
    pub block: String,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThemeEngine {
    pub active_theme: ThemeMetadata,
    pub template_paths: Vec<PathBuf>,
    pub static_paths: Vec<PathBuf>,
    pub variables: HashMap<String, serde_yaml::Value>,
}

impl ThemeEngine {
    pub fn new(ssg_config: &SsgConfig) -> Result<Self> {
        let theme_dir = PathBuf::from("themes");
        let theme_name = ssg_config.theme.name.clone();

        let active_theme = load_theme_metadata(&theme_dir, &theme_name)?;
        let parent_theme_name = active_theme.parent.clone();
        let parent_theme = if let Some(ref parent_name) = parent_theme_name {
            Some(load_theme_metadata(&theme_dir, parent_name)?)
        } else {
            None
        };

        let template_paths = resolve_template_paths(&theme_dir, &theme_name, &parent_theme_name)?;
        let static_paths = resolve_static_paths(&theme_dir, &theme_name, &parent_theme_name);
        let variables = merge_variables(&active_theme, &parent_theme, &ssg_config.theme.variables);

        Ok(Self {
            active_theme,
            template_paths,
            static_paths,
            variables,
        })
    }

    pub fn create_tera_engine(&self) -> Result<Tera> {
        let primary_path = self
            .template_paths
            .first()
            .ok_or_else(|| anyhow::anyhow!("No template paths available"))?;

        let glob_pattern = format!("{}/**/*.html", primary_path.display());
        let mut tera = Tera::new(&glob_pattern)
            .context(format!("Failed to load templates from {:?}", primary_path))?;

        for path in self.template_paths.iter().skip(1) {
            if path.exists() {
                let fallback_pattern = format!("{}/**/*.html", path.display());
                match Tera::new(&fallback_pattern) {
                    Ok(fallback_tera) => {
                        tera.extend(&fallback_tera)?;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        }

        validate_required_templates(&tera, &self.active_theme)?;

        Ok(tera)
    }

    pub fn get_template_variables(&self) -> HashMap<String, serde_yaml::Value> {
        self.variables.clone()
    }

    pub fn get_theme_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        info.insert("name".to_string(), self.active_theme.name.clone());
        info.insert("version".to_string(), self.active_theme.version.clone());
        info.insert("author".to_string(), self.active_theme.author.clone());
        info
    }

    pub fn copy_theme_assets(&self, output_dir: &Path) -> Result<()> {
        for static_path in &self.static_paths {
            copy_dir_all(static_path, output_dir)?;
        }
        Ok(())
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn load_theme_metadata(theme_dir: &Path, theme_name: &str) -> Result<ThemeMetadata> {
    let theme_path = theme_dir.join(theme_name);
    let metadata_path = theme_path.join("theme.yaml");

    if !theme_path.exists() {
        if theme_name == "default" && Path::new("templates").exists() {
            return get_default_theme_metadata();
        }
        anyhow::bail!(
            "Theme '{}' not found at {:?}. Available themes should be in the {} directory.",
            theme_name,
            theme_path,
            theme_dir.display()
        );
    }

    if !metadata_path.exists() {
        anyhow::bail!(
            "Theme '{}' is missing theme.yaml metadata file at {:?}",
            theme_name,
            metadata_path
        );
    }

    let content = fs::read_to_string(&metadata_path)
        .context(format!("Failed to read {:?}", metadata_path))?;

    let mut metadata: ThemeMetadata =
        serde_yaml::from_str(&content).context(format!("Failed to parse {:?}", metadata_path))?;

    if metadata.name.is_empty() {
        metadata.name = theme_name.to_string();
    }

    Ok(metadata)
}

fn get_default_theme_metadata() -> Result<ThemeMetadata> {
    Ok(ThemeMetadata {
        name: "default".to_string(),
        version: "1.0.0".to_string(),
        author: "".to_string(),
        description: "Legacy template directory".to_string(),
        parent: None,
        variables: HashMap::new(),
        hooks: vec![],
        required_templates: vec![
            "base.html".to_string(),
            "post.html".to_string(),
            "index.html".to_string(),
        ],
    })
}

fn resolve_template_paths(
    theme_dir: &Path,
    theme_name: &str,
    parent_theme_name: &Option<String>,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    let active_path = theme_dir.join(theme_name);
    if active_path.exists() {
        paths.push(active_path);
    }

    if let Some(parent_name) = parent_theme_name {
        let parent_path = theme_dir.join(parent_name);
        if parent_path.exists() {
            paths.push(parent_path);
        }
    }

    if paths.is_empty() {
        anyhow::bail!(
            "No template directories found. Expected theme at {:?}",
            theme_dir.join(theme_name)
        );
    }

    Ok(paths)
}

fn resolve_static_paths(
    theme_dir: &Path,
    theme_name: &str,
    parent_theme_name: &Option<String>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(parent_name) = parent_theme_name {
        let parent_static = theme_dir.join(parent_name).join("static");
        if parent_static.exists() && parent_static.is_dir() {
            paths.push(parent_static);
        }
    }

    let active_static = theme_dir.join(theme_name).join("static");
    if active_static.exists() && active_static.is_dir() {
        paths.push(active_static);
    }

    paths
}

fn merge_variables(
    active_theme: &ThemeMetadata,
    parent_theme: &Option<ThemeMetadata>,
    site_overrides: &HashMap<String, serde_yaml::Value>,
) -> HashMap<String, serde_yaml::Value> {
    let mut variables = HashMap::new();

    if let Some(parent) = parent_theme {
        for (key, value) in &parent.variables {
            variables.insert(key.clone(), value.clone());
        }
    }

    for (key, value) in &active_theme.variables {
        variables.insert(key.clone(), value.clone());
    }

    for (key, value) in site_overrides {
        variables.insert(key.clone(), value.clone());
    }

    variables
}

fn validate_required_templates(tera: &Tera, theme: &ThemeMetadata) -> Result<()> {
    let missing_templates: Vec<&String> = theme
        .required_templates
        .iter()
        .filter(|template| {
            !tera
                .get_template_names()
                .any(|name| name == template.as_str())
        })
        .collect();

    if !missing_templates.is_empty() {
        anyhow::bail!(
            "Theme '{}' is missing required templates: {:?}",
            theme.name,
            missing_templates
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_variables() {
        let parent = ThemeMetadata {
            name: "parent".to_string(),
            version: "1.0.0".to_string(),
            author: "".to_string(),
            description: "".to_string(),
            parent: None,
            variables: {
                let mut vars = HashMap::new();
                vars.insert(
                    "color".to_string(),
                    serde_yaml::Value::String("red".to_string()),
                );
                vars.insert(
                    "font".to_string(),
                    serde_yaml::Value::String("Arial".to_string()),
                );
                vars
            },
            hooks: vec![],
            required_templates: vec![],
        };

        let child = ThemeMetadata {
            name: "child".to_string(),
            version: "1.0.0".to_string(),
            author: "".to_string(),
            description: "".to_string(),
            parent: Some("parent".to_string()),
            variables: {
                let mut vars = HashMap::new();
                vars.insert(
                    "color".to_string(),
                    serde_yaml::Value::String("blue".to_string()),
                );
                vars
            },
            hooks: vec![],
            required_templates: vec![],
        };

        let site_overrides = {
            let mut vars = HashMap::new();
            vars.insert(
                "font".to_string(),
                serde_yaml::Value::String("Helvetica".to_string()),
            );
            vars
        };

        let merged = merge_variables(&child, &Some(parent), &site_overrides);

        assert_eq!(
            merged.get("color").unwrap(),
            &serde_yaml::Value::String("blue".to_string())
        );
        assert_eq!(
            merged.get("font").unwrap(),
            &serde_yaml::Value::String("Helvetica".to_string())
        );
    }
}
