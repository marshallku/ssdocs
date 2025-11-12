use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::types::Category;

/// Discover all categories from the content directory
pub fn discover_categories(content_dir: &Path) -> Result<Vec<Category>> {
    let mut categories = Vec::new();

    for entry in fs::read_dir(content_dir)
        .with_context(|| format!("Failed to read content directory: {}", content_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        // Skip non-directories and hidden directories
        if !path.is_dir() || is_hidden(&path) {
            continue;
        }

        // Skip if no markdown files in directory
        if !has_markdown_files(&path)? {
            continue;
        }

        let slug = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid directory name: {}", path.display()))?
            .to_string();

        // Load category metadata
        let category = load_category_metadata(&path, &slug)?;
        categories.push(category);
    }

    // Sort by index, then alphabetically
    categories.sort_by(|a, b| match a.index.cmp(&b.index) {
        std::cmp::Ordering::Equal => a.name.cmp(&b.name),
        other => other,
    });

    Ok(categories)
}

/// Load category metadata from .category.yaml or create default
fn load_category_metadata(dir: &Path, slug: &str) -> Result<Category> {
    let metadata_path = dir.join(".category.yaml");

    let mut category = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path).with_context(|| {
            format!(
                "Failed to read category metadata: {}",
                metadata_path.display()
            )
        })?;
        serde_yaml::from_str::<Category>(&content).with_context(|| {
            format!(
                "Failed to parse .category.yaml in '{}'",
                dir.display()
            )
        })?
    } else {
        // Default category
        Category {
            slug: slug.to_string(),
            name: capitalize(slug),
            description: String::new(),
            index: 999,
            hidden: false,
            icon: None,
            color: None,
            cover_image: None,
        }
    };

    // Always set slug from directory name (overrides any value in .yaml)
    category.slug = slug.to_string();

    Ok(category)
}

/// Check if a path is hidden (starts with . or _)
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.') || n.starts_with('_'))
        .unwrap_or(false)
}

/// Check if a directory contains any markdown files
fn has_markdown_files(dir: &Path) -> Result<bool> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" || ext == "markdown" {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Capitalize first letter of a string
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Validate that a category slug exists
pub fn validate_category(slug: &str, categories: &[Category]) -> bool {
    categories.iter().any(|c| c.slug == slug)
}

/// Get a category by its slug
pub fn get_category_by_slug<'a>(slug: &str, categories: &'a [Category]) -> Option<&'a Category> {
    categories.iter().find(|c| c.slug == slug)
}

/// Get all visible categories (not hidden)
pub fn get_visible_categories(categories: &[Category]) -> Vec<&Category> {
    categories.iter().filter(|c| !c.hidden).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_categories() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        // Create structure
        fs::create_dir(content.join("dev")).unwrap();
        fs::write(content.join("dev/post.md"), "# Test").unwrap();

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].slug, "dev");
        assert_eq!(categories[0].name, "Dev");
    }

    #[test]
    fn test_load_category_metadata() {
        let temp = TempDir::new().unwrap();
        let cat_dir = temp.path().join("dev");
        fs::create_dir(&cat_dir).unwrap();

        let yaml = r#"
name: Development
description: Tech posts
index: 0
        "#;
        fs::write(cat_dir.join(".category.yaml"), yaml).unwrap();

        let category = load_category_metadata(&cat_dir, "dev").unwrap();
        assert_eq!(category.name, "Development");
        assert_eq!(category.description, "Tech posts");
        assert_eq!(category.index, 0);
    }

    #[test]
    fn test_hidden_directories_ignored() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        fs::create_dir(content.join(".hidden")).unwrap();
        fs::write(content.join(".hidden/post.md"), "# Test").unwrap();

        fs::create_dir(content.join("_private")).unwrap();
        fs::write(content.join("_private/post.md"), "# Test").unwrap();

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories.len(), 0);
    }

    #[test]
    fn test_category_sorting() {
        let temp = TempDir::new().unwrap();
        let content = temp.path();

        // Create categories with different indices
        for (name, index) in &[("zzz", 0), ("aaa", 2), ("mmm", 1)] {
            let dir = content.join(name);
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("post.md"), "# Test").unwrap();
            fs::write(dir.join(".category.yaml"), format!("index: {}", index)).unwrap();
        }

        let categories = discover_categories(content).unwrap();
        assert_eq!(categories[0].slug, "zzz"); // index 0
        assert_eq!(categories[1].slug, "mmm"); // index 1
        assert_eq!(categories[2].slug, "aaa"); // index 2
    }

    #[test]
    fn test_validate_category() {
        let categories = vec![
            Category {
                slug: "dev".to_string(),
                name: "Development".to_string(),
                description: String::new(),
                index: 0,
                hidden: false,
                icon: None,
                color: None,
                cover_image: None,
            },
            Category {
                slug: "blog".to_string(),
                name: "Blog".to_string(),
                description: String::new(),
                index: 1,
                hidden: false,
                icon: None,
                color: None,
                cover_image: None,
            },
        ];

        assert!(validate_category("dev", &categories));
        assert!(validate_category("blog", &categories));
        assert!(!validate_category("invalid", &categories));
    }

    #[test]
    fn test_get_visible_categories() {
        let categories = vec![
            Category {
                slug: "dev".to_string(),
                name: "Development".to_string(),
                description: String::new(),
                index: 0,
                hidden: false,
                icon: None,
                color: None,
                cover_image: None,
            },
            Category {
                slug: "drafts".to_string(),
                name: "Drafts".to_string(),
                description: String::new(),
                index: 1,
                hidden: true,
                icon: None,
                color: None,
                cover_image: None,
            },
        ];

        let visible = get_visible_categories(&categories);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].slug, "dev");
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("dev"), "Dev");
        assert_eq!(capitalize("tutorials"), "Tutorials");
        assert_eq!(capitalize(""), "");
    }
}
