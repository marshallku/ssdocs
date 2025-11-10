use crate::types::{Frontmatter, Post};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct Parser;

impl Parser {
    /// Parse a markdown file and extract frontmatter + content
    pub fn parse_file(path: &Path) -> Result<Post> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let (frontmatter_str, markdown) = Self::split_frontmatter(&content)?;
        let frontmatter = Self::parse_frontmatter(frontmatter_str)?;
        let slug = Self::path_to_slug(path)?;

        Ok(Post {
            slug,
            frontmatter,
            content: markdown.to_string(),
            rendered_html: None,
        })
    }

    /// Split content into frontmatter and markdown
    /// Expected format:
    /// ---
    /// frontmatter here
    /// ---
    /// markdown here
    fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid frontmatter format. Expected:\n---\nfrontmatter\n---\ncontent");
        }

        // parts[0] is empty string before first ---
        // parts[1] is frontmatter
        // parts[2] is content
        Ok((parts[1].trim(), parts[2].trim()))
    }

    /// Parse YAML frontmatter into Frontmatter struct
    fn parse_frontmatter(yaml: &str) -> Result<Frontmatter> {
        serde_yaml::from_str(yaml)
            .context("Failed to parse frontmatter YAML")
    }

    /// Convert file path to slug
    /// Example: content/posts/dev/my-post.md → my-post
    fn path_to_slug(path: &Path) -> Result<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid file path: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_frontmatter() {
        let content = r#"---
title: Test Post
---
Content here"#;

        let (fm, content) = Parser::split_frontmatter(content).unwrap();
        assert!(fm.contains("title: Test Post"));
        assert_eq!(content, "Content here");
    }

    #[test]
    fn test_split_frontmatter_multiline() {
        let content = r#"---
title: Test
date: 2025-11-11T10:00:00Z
---
# Heading

Content with multiple lines"#;

        let (fm, content) = Parser::split_frontmatter(content).unwrap();
        assert!(fm.contains("title: Test"));
        assert!(content.starts_with("# Heading"));
    }

    #[test]
    fn test_path_to_slug() {
        let path = Path::new("content/posts/dev/hello-world.md");
        let slug = Parser::path_to_slug(path).unwrap();
        assert_eq!(slug, "hello-world");
    }
}
