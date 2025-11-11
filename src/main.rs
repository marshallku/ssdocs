mod types;
mod parser;
mod renderer;
mod generator;
mod cache;

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use std::path::Path;
use walkdir::WalkDir;

use crate::cache::{BuildCache, hash_file};
use crate::generator::Generator;
use crate::parser::Parser;
use crate::renderer::Renderer;
use crate::types::Config;

#[derive(ClapParser)]
#[command(name = "ssg")]
#[command(about = "A blazing-fast static site generator for marshallku blog")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the site
    Build {
        /// Build only changed files (incremental build)
        #[arg(short, long)]
        incremental: bool,

        /// Build a specific post
        #[arg(short, long)]
        post: Option<String>,
    },

    /// Watch for changes and rebuild
    Watch {
        /// Port for dev server
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Create a new post
    New {
        /// Category (dev, chat, gallery, notice)
        category: String,

        /// Post title
        title: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { incremental, post } => {
            if let Some(post_path) = post {
                build_single_post(&post_path)?;
            } else if incremental {
                println!("Note: Incremental build uses cache to skip unchanged files");
                build_all(true)?;
            } else {
                build_all(false)?;
            }
        }
        Commands::Watch { port } => {
            println!("Watch mode not yet implemented");
            println!("Will watch for changes and rebuild automatically on port {}", port);
        }
        Commands::New { category, title } => {
            create_new_post(&category, &title)?;
        }
    }

    Ok(())
}

fn build_all(use_cache: bool) -> Result<()> {
    println!("Building site...\n");

    let config = Config::default();
    let renderer = Renderer::new();
    let generator = Generator::new(config.clone())?;
    let mut cache = if use_cache {
        BuildCache::load()?
    } else {
        BuildCache::new()
    };

    let posts_dir = Path::new(&config.content_dir);

    if !posts_dir.exists() {
        anyhow::bail!(
            "Content directory '{}' does not exist. Create it first with: mkdir -p {}",
            config.content_dir,
            config.content_dir
        );
    }

    let mut built_count = 0;
    let mut skipped_count = 0;

    // Find all markdown files
    for entry in WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let path = entry.path();
        let file_hash = hash_file(path)?;

        if use_cache && !cache.needs_rebuild(path, &file_hash) {
            println!("⏭  Skipping (unchanged): {}", path.display());
            skipped_count += 1;
            continue;
        }

        println!("🔨 Building: {}", path.display());

        // Parse post
        let mut post = Parser::parse_file(path)?;

        // Skip drafts
        if post.frontmatter.draft {
            println!("   ⚠  Draft - skipping output");
            skipped_count += 1;
            continue;
        }

        // Render markdown to HTML
        let html = renderer.render_markdown(&post.content);
        post.rendered_html = Some(html);

        // Generate HTML file
        let output_path = generator.generate_post(&post)?;

        // Update cache
        cache.update_entry(
            path,
            file_hash,
            "template_hash_placeholder".to_string(),
            output_path.to_string_lossy().to_string(),
        );

        built_count += 1;
    }

    // Save cache
    if use_cache {
        cache.save()?;
    }

    // Copy static assets
    generator.copy_static_assets()?;

    println!("\n✅ Build complete!");
    println!("   Built: {}", built_count);
    if use_cache {
        println!("   Skipped: {}", skipped_count);
    }

    Ok(())
}

fn build_single_post(post_path: &str) -> Result<()> {
    println!("Building single post: {}\n", post_path);

    let config = Config::default();
    let renderer = Renderer::new();
    let generator = Generator::new(config.clone())?;

    let path = Path::new(post_path);

    if !path.exists() {
        anyhow::bail!("Post file not found: {}", post_path);
    }

    // Parse post
    let mut post = Parser::parse_file(path)?;

    if post.frontmatter.draft {
        println!("⚠  This is a draft post");
    }

    // Render markdown to HTML
    let html = renderer.render_markdown(&post.content);
    post.rendered_html = Some(html);

    // Generate HTML file
    let output_path = generator.generate_post(&post)?;

    println!("\n✅ Built: {}", output_path.display());

    Ok(())
}

fn create_new_post(category: &str, title: &str) -> Result<()> {
    // Validate category
    let valid_categories = ["dev", "chat", "gallery", "notice"];
    if !valid_categories.contains(&category) {
        anyhow::bail!(
            "Invalid category '{}'. Must be one of: {}",
            category,
            valid_categories.join(", ")
        );
    }

    // Generate slug from title
    let slug = title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    let filename = format!("content/posts/{}/{}.md", category, slug);

    // Check if file already exists
    if Path::new(&filename).exists() {
        anyhow::bail!("Post already exists: {}", filename);
    }

    // Create frontmatter
    let content = format!(
        r#"---
title: "{}"
date: {}
category: {}
tags: []
draft: false
---

Write your post here...
"#,
        title,
        chrono::Utc::now().to_rfc3339(),
        category
    );

    // Create directory if needed
    std::fs::create_dir_all(format!("content/posts/{}", category))?;

    // Write file
    std::fs::write(&filename, content)?;

    println!("✅ Created: {}", filename);
    println!("   Title: {}", title);
    println!("   Category: {}", category);
    println!("   Slug: {}", slug);

    Ok(())
}
