use crate::config::SsgConfig;
use crate::theme::ThemeEngine;
use crate::types::Post;
use anyhow::Result;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

/// Flattened config for template context (backward compatibility)
#[derive(Debug, Clone, Serialize)]
struct TemplateConfig<'a> {
    site_title: &'a str,
    site_url: &'a str,
    author: &'a str,
}

pub struct Generator {
    tera: Tera,
    config: SsgConfig,
    theme_engine: ThemeEngine,
    theme_variables: HashMap<String, serde_yaml::Value>,
    theme_info: HashMap<String, String>,
}

impl Generator {
    pub fn new(config: SsgConfig) -> Result<Self> {
        let theme_engine = ThemeEngine::new(&config)?;
        let tera = theme_engine.create_tera_engine()?;
        let theme_variables = theme_engine.get_template_variables();
        let theme_info = theme_engine.get_theme_info();

        Ok(Self {
            tera,
            config,
            theme_engine,
            theme_variables,
            theme_info,
        })
    }

    pub fn generate_post(&self, post: &Post) -> Result<PathBuf> {
        let html = post
            .rendered_html
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Post not rendered: {}", post.slug))?;

        let template_config = TemplateConfig {
            site_title: &self.config.site.title,
            site_url: &self.config.site.url,
            author: &self.config.site.author,
        };

        let mut context = TeraContext::new();
        context.insert("post", &post.frontmatter);
        context.insert("slug", &post.slug);
        context.insert("content", html);
        context.insert("config", &template_config);

        // Add theme context
        context.insert("theme_variables", &self.theme_variables);
        context.insert("theme_info", &self.theme_info);

        let output = self.tera.render("post.html", &context)?;

        let output_path = self.get_post_path(post);
        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output)?;

        Ok(output_path)
    }

    fn get_post_path(&self, post: &Post) -> PathBuf {
        PathBuf::from(&self.config.build.output_dir)
            .join(&post.frontmatter.category)
            .join(&post.slug)
            .join("index.html")
    }

    pub fn copy_static_assets(&self) -> Result<()> {
        let dst = Path::new(&self.config.build.output_dir);

        self.theme_engine.copy_theme_assets(dst)?;
        if !self.theme_engine.static_paths.is_empty() {
            println!("📦 Copied theme static assets");
        }

        let src = Path::new("static");
        if src.exists() {
            Self::copy_dir_all(src, dst)?;
            println!("📦 Copied static assets");
        }

        Ok(())
    }

    fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if ty.is_dir() {
                Self::copy_dir_all(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }
}
