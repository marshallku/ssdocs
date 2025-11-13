use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::theme::ThemeEngine;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tera::{Context as TeraContext, Tera};

/// Flattened config for template context (backward compatibility)
#[derive(Debug, Clone, Serialize)]
struct TemplateConfig<'a> {
    site_title: &'a str,
    site_url: &'a str,
    author: &'a str,
}

pub struct IndexGenerator {
    tera: Tera,
    config: SsgConfig,
    theme_variables: HashMap<String, serde_yaml::Value>,
    theme_info: HashMap<String, String>,
}

impl IndexGenerator {
    pub fn new(config: SsgConfig) -> Result<Self> {
        let theme_engine = ThemeEngine::new(&config)?;
        let tera = theme_engine.create_tera_engine()?;
        let theme_variables = theme_engine.get_template_variables();
        let theme_info = theme_engine.get_theme_info();

        Ok(Self {
            tera,
            config,
            theme_variables,
            theme_info,
        })
    }

    /// Generate all indices (homepage, categories, tags)
    pub fn generate_all(&self, metadata: &MetadataCache) -> Result<()> {
        println!("\n📑 Generating indices...");

        // Generate homepage
        self.generate_homepage(metadata)?;

        // Generate category pages (from discovered categories, including hidden)
        let category_count = metadata.get_category_info().len();
        for category in metadata.get_category_info() {
            self.generate_category_page(category, metadata)?;
        }

        // Generate tag pages
        for tag in metadata.get_tags() {
            self.generate_tag_page(&tag, metadata)?;
        }

        // Generate tags overview page
        self.generate_tags_overview(metadata)?;

        println!("   ✓ Homepage");
        println!("   ✓ {} category pages", category_count);
        println!("   ✓ {} tag pages", metadata.get_tags().len());

        Ok(())
    }

    /// Generate homepage with recent posts
    fn generate_homepage(&self, metadata: &MetadataCache) -> Result<()> {
        let recent_posts = metadata.get_recent_posts(10);

        // Get visible categories for navigation
        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
        };

        let mut context = TeraContext::new();
        context.insert("posts", &recent_posts);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        // Add theme context
        context.insert("theme_variables", &self.theme_variables);
        context.insert("theme_info", &self.theme_info);

        let output = self.tera.render("index.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir).join("index.html");

        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate category page
    fn generate_category_page(
        &self,
        category_info: &crate::types::Category,
        metadata: &MetadataCache,
    ) -> Result<()> {
        let mut posts = metadata.get_posts_by_category(&category_info.slug);

        // Sort by date, descending
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let post_count = metadata.categories.get(&category_info.slug).unwrap_or(&0);

        // Get visible categories for navigation
        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
        };

        let mut context = TeraContext::new();
        context.insert("category", category_info);
        context.insert("posts", &posts);
        context.insert("post_count", post_count);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        // Add theme context
        context.insert("theme_variables", &self.theme_variables);
        context.insert("theme_info", &self.theme_info);

        let output = self.tera.render("category.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir)
            .join(&category_info.slug)
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate tag page
    fn generate_tag_page(&self, tag: &str, metadata: &MetadataCache) -> Result<()> {
        let mut posts = metadata.get_posts_by_tag(tag);

        // Sort by date, descending
        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let post_count = metadata.tags.get(tag).unwrap_or(&0);

        // Get visible categories for navigation
        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
        };

        let mut context = TeraContext::new();
        context.insert("tag", tag);
        context.insert("posts", &posts);
        context.insert("post_count", post_count);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        // Add theme context
        context.insert("theme_variables", &self.theme_variables);
        context.insert("theme_info", &self.theme_info);

        let output = self.tera.render("tag.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir)
            .join("tag")
            .join(tag)
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }

    /// Generate tags overview page (list of all tags)
    fn generate_tags_overview(&self, metadata: &MetadataCache) -> Result<()> {
        let mut tags_with_counts: Vec<_> = metadata.tags.iter().collect();
        tags_with_counts.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count, descending

        // Get visible categories for navigation
        let visible_categories: Vec<_> = metadata
            .get_category_info()
            .iter()
            .filter(|c| !c.hidden)
            .collect();

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
        };

        let mut context = TeraContext::new();
        context.insert("tags", &tags_with_counts);
        context.insert("categories", &visible_categories);
        context.insert("config", &template_config);

        // Add theme context
        context.insert("theme_variables", &self.theme_variables);
        context.insert("theme_info", &self.theme_info);

        let output = self.tera.render("tags.html", &context)?;
        let output_path = PathBuf::from(&self.config.build.output_dir)
            .join("tags")
            .join("index.html");

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(())
    }
}
