use crate::types::{Frontmatter, Page, PageFrontmatter, Post};
use anyhow::{Context, Result};
use blake3;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::fs;
use std::path::Path;

// Define characters that should NOT be percent-encoded
// https://url.spec.whatwg.org/#path-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const PATH: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}');

pub struct Parser;

impl Parser {
    pub fn parse_file(path: &Path) -> Result<Post> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let (frontmatter_str, markdown) = Self::split_frontmatter(&content)?;
        let frontmatter = Self::parse_frontmatter(frontmatter_str)?;
        let raw_slug = Self::path_to_slug(path)?;
        let slug = Self::encode_slug(&raw_slug);
        let category = Self::extract_category(path)?;

        Ok(Post {
            slug,
            category,
            frontmatter,
            content: markdown.to_string(),
            rendered_html: None,
        })
    }

    fn encode_slug(slug: &str) -> String {
        // Percent-encode non-ASCII characters for filesystem paths and URLs
        // This keeps ASCII letters, numbers, hyphens, underscores, and dots as-is
        let encoded = utf8_percent_encode(slug, PATH).to_string();

        // Filesystem limit is usually 255 bytes, keep some margin
        const MAX_LEN: usize = 200;
        if encoded.len() > MAX_LEN {
            // If too long, take first 180 chars + hash of full string for uniqueness
            let hash = blake3::hash(encoded.as_bytes());
            format!("{}-{}", &encoded[..180], &hash.to_hex()[..16])
        } else {
            encoded
        }
    }

    fn extract_category(path: &Path) -> Result<String> {
        let components: Vec<_> = path.components().collect();

        for i in 0..components.len() {
            if let std::path::Component::Normal(comp) = components[i] {
                if comp == "posts" && i + 1 < components.len() {
                    if let std::path::Component::Normal(category) = components[i + 1] {
                        return category.to_str().map(|s| s.to_string()).ok_or_else(|| {
                            anyhow::anyhow!("Invalid category name in path: {}", path.display())
                        });
                    }
                }
            }
        }

        anyhow::bail!("Could not extract category from path: {}. Expected path format: content/posts/<category>/...", path.display())
    }

    pub fn parse_page_file(path: &Path) -> Result<Page> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let slug = Self::path_to_slug(path)?;

        if content.trim_start().starts_with("---") {
            let (frontmatter_str, markdown) = Self::split_frontmatter(&content)?;
            let frontmatter = Self::parse_page_frontmatter(frontmatter_str)?;

            Ok(Page {
                slug,
                frontmatter,
                content: markdown.to_string(),
                rendered_html: None,
            })
        } else {
            Ok(Page {
                slug: slug.clone(),
                frontmatter: PageFrontmatter {
                    title: slug.replace('-', " "),
                    description: None,
                    draft: false,
                },
                content: content.to_string(),
                rendered_html: None,
            })
        }
    }

    fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid frontmatter format. Expected:\n---\nfrontmatter\n---\ncontent");
        }

        Ok((parts[1].trim(), parts[2].trim()))
    }

    fn parse_frontmatter(yaml: &str) -> Result<Frontmatter> {
        serde_yaml::from_str(yaml).context("Failed to parse frontmatter YAML")
    }

    fn parse_page_frontmatter(yaml: &str) -> Result<PageFrontmatter> {
        serde_yaml::from_str(yaml).context("Failed to parse page frontmatter YAML")
    }

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
