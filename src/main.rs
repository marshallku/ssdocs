mod cache;
mod category;
mod config;
mod feeds;
mod generator;
mod indices;
mod metadata;
mod parallel;
mod parser;
mod plugin;
mod plugins;
mod renderer;
mod search;
mod shortcodes;
mod slug;
mod theme;
mod types;

use anyhow::Result;
use clap::{Parser as ClapParser, Subcommand};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use walkdir::WalkDir;

use crate::cache::{hash_directory, hash_file, BuildCache};
use crate::category::{discover_categories, validate_category};
use crate::config::load_config;
use crate::feeds::FeedGenerator;
use crate::generator::Generator;
use crate::indices::IndexGenerator;
use crate::metadata::MetadataCache;
use crate::parser::Parser;
use crate::parallel::{get_thread_count, BuildProgress, BuildResult, SkipReason, WorkQueue, WorkerPool};
use crate::plugin::{PluginContext, PluginManager};
use crate::plugins::RelatedPostsPlugin;
use crate::renderer::Renderer;
use crate::search::SearchIndexGenerator;
use crate::shortcodes::ShortcodeRegistry;

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

        /// Use parallel processing for faster builds
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        parallel: bool,
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
        Commands::Build {
            incremental,
            post,
            parallel,
        } => {
            if let Some(post_path) = post {
                build_single_post(&post_path)?;
            } else if parallel {
                if incremental {
                    println!("Note: Incremental build uses cache to skip unchanged files");
                }
                build_all_parallel(incremental)?;
            } else if incremental {
                println!("Note: Incremental build uses cache to skip unchanged files");
                build_all(true)?;
            } else {
                build_all(false)?;
            }
        }
        Commands::Watch { port } => {
            watch_mode(port)?;
        }
        Commands::New { category, title } => {
            create_new_post(&category, &title)?;
        }
    }

    Ok(())
}

fn build_all(use_cache: bool) -> Result<()> {
    println!("Building site...\n");

    let config = load_config()?;
    let renderer = Renderer::new();
    let mut shortcode_registry = ShortcodeRegistry::new();
    let generator = Generator::new(config.clone())?;
    let mut cache = if use_cache {
        BuildCache::load()?
    } else {
        BuildCache::new()
    };
    let mut metadata = if use_cache {
        MetadataCache::load().unwrap_or_else(|_| MetadataCache::new())
    } else {
        MetadataCache::new()
    };

    // Initialize plugin system
    let mut plugin_manager = PluginManager::new();
    plugin_manager.register(Box::new(RelatedPostsPlugin::new()));
    plugin_manager.init_all(&config)?;

    // Register plugin shortcodes
    plugin_manager.register_shortcodes(&mut shortcode_registry);

    println!(
        "🔌 Loaded plugins: {}",
        plugin_manager.list_plugins().join(", ")
    );

    let posts_dir = Path::new(&config.build.content_dir);

    if !posts_dir.exists() {
        anyhow::bail!(
            "Content directory '{}' does not exist. Create it first with: mkdir -p {}",
            config.build.content_dir,
            config.build.content_dir
        );
    }

    let template_hash = hash_directory(Path::new(&format!("themes/{}", config.theme.name)))?;

    let categories = discover_categories(posts_dir)?;
    if categories.is_empty() {
        eprintln!("⚠️  Warning: No categories found in content directory");
        eprintln!("   Create a category by adding a subdirectory with markdown files:");
        eprintln!("   mkdir -p {}/dev", config.build.content_dir);
    }
    metadata.set_category_info(categories);

    let mut built_count = 0;
    let mut skipped_count = 0;

    for entry in WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let path = entry.path();
        let file_hash = hash_file(path)?;

        if use_cache && !cache.needs_rebuild(path, &file_hash, &template_hash) {
            println!("⏭  Skipping (unchanged): {}", path.display());
            skipped_count += 1;
            continue;
        }

        println!("🔨 Building: {}", path.display());

        let mut post = Parser::parse_file(path)?;

        if post.frontmatter.draft {
            println!("   ⚠  Draft - skipping output");
            skipped_count += 1;
            continue;
        }

        // Create plugin context
        let plugin_ctx = PluginContext {
            config: &config,
            metadata: &metadata,
        };

        // Plugin hook: after parsing
        plugin_manager.on_post_parsed(&mut post, &plugin_ctx)?;

        // Process shortcodes before markdown rendering
        let processed_content = shortcode_registry.process(&post.content)?;

        let base_path = format!("{}", post.category);
        let mut html = renderer.render_markdown_with_components(
            &processed_content,
            generator.get_tera(),
            &base_path,
        )?;

        // Plugin hook: after rendering
        plugin_manager.on_post_rendered(&mut post, &mut html, &plugin_ctx)?;

        post.rendered_html = Some(html);

        // Collect plugin template data
        let plugin_data = plugin_manager.template_context_post(&post, &plugin_ctx)?;

        let output_path = generator.generate_post(&post, &plugin_data)?;

        cache.update_entry(
            path,
            file_hash,
            template_hash.clone(),
            output_path.to_string_lossy().to_string(),
        );

        metadata.upsert_post(
            post.slug.clone(),
            post.category.clone(),
            post.frontmatter.clone(),
        );

        built_count += 1;
    }

    if use_cache {
        cache.save()?;
    }
    metadata.save()?;

    let pages_dir = Path::new("content/pages");
    if pages_dir.exists() {
        println!("\n📄 Building pages...");
        let mut pages_built = 0;

        for entry in WalkDir::new(pages_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let path = entry.path();
            println!("🔨 Building page: {}", path.display());

            let mut page = Parser::parse_page_file(path)?;

            if page.frontmatter.draft {
                println!("   ⚠  Draft - skipping output");
                continue;
            }

            // Process shortcodes before markdown rendering
            let processed_content = shortcode_registry.process(&page.content)?;

            let html = renderer.render_markdown_with_components(
                &processed_content,
                generator.get_tera(),
                &page.slug,
            )?;
            page.rendered_html = Some(html);

            // Collect plugin template data for pages
            let plugin_ctx = PluginContext {
                config: &config,
                metadata: &metadata,
            };
            let plugin_data = plugin_manager.template_context_page(&page, &plugin_ctx)?;

            let output_path = generator.generate_page(&page, &plugin_data)?;
            println!("   ✓ {}", output_path.display());

            pages_built += 1;
        }

        if pages_built > 0 {
            println!("✅ Built {} page(s)", pages_built);
        }
    }

    let index_generator = IndexGenerator::new(config.clone())?;
    index_generator.generate_all(&metadata, &plugin_manager)?;

    println!("📄 Generating RSS feeds...");
    FeedGenerator::generate_all_feeds(
        &config,
        &metadata,
        posts_dir,
        Path::new(&config.build.output_dir),
    )?;

    if config.build.search.enabled {
        let search_generator = SearchIndexGenerator::new(config.clone());
        search_generator.generate(&metadata)?;
    }

    println!("🎨 Generating syntax highlighting CSS...");
    let css_dir = Path::new(&config.build.output_dir).join("css");
    std::fs::create_dir_all(&css_dir)?;
    renderer.write_syntax_css(css_dir.join("syntax.css"))?;

    generator.copy_content_assets()?;
    generator.copy_static_assets()?;

    println!("\n✅ Build complete!");
    println!("   Built: {}", built_count);
    if use_cache {
        println!("   Skipped: {}", skipped_count);
    }
    println!("   Categories: {}", metadata.get_categories().len());
    println!("   Tags: {}", metadata.get_tags().len());

    Ok(())
}

fn build_all_parallel(use_cache: bool) -> Result<()> {
    let start_time = std::time::Instant::now();
    let num_threads = get_thread_count();
    println!("Building site with {} threads...\n", num_threads);

    let config = Arc::new(load_config()?);
    let posts_dir = Path::new(&config.build.content_dir);

    if !posts_dir.exists() {
        anyhow::bail!(
            "Content directory '{}' does not exist",
            config.build.content_dir
        );
    }

    let template_hash = Arc::new(hash_directory(Path::new(&format!(
        "themes/{}",
        config.theme.name
    )))?);

    let categories = discover_categories(posts_dir)?;
    let mut metadata = if use_cache {
        MetadataCache::load().unwrap_or_else(|_| MetadataCache::new())
    } else {
        MetadataCache::new()
    };
    metadata.set_category_info(categories);

    let cache = Arc::new(Mutex::new(if use_cache {
        BuildCache::load()?
    } else {
        BuildCache::new()
    }));

    // Initialize plugin system
    let mut plugin_manager = PluginManager::new();
    plugin_manager.register(Box::new(RelatedPostsPlugin::new()));
    plugin_manager.init_all(&config)?;

    let mut shortcode_registry = ShortcodeRegistry::new();
    plugin_manager.register_shortcodes(&mut shortcode_registry);
    let shortcode_registry = Arc::new(shortcode_registry);

    println!(
        "🔌 Loaded plugins: {}",
        plugin_manager.list_plugins().join(", ")
    );

    // Collect all post files
    let file_paths: Vec<PathBuf> = WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        .map(|e| e.path().to_path_buf())
        .collect();

    let progress = Arc::new(BuildProgress::new());

    // Set up work queue and results channel
    let work_queue = WorkQueue::new();
    let work_rx = work_queue.get_receiver();
    let (result_tx, result_rx) = mpsc::channel();

    // Send all work to queue
    for path in file_paths {
        work_queue.send(path)?;
    }
    work_queue.close();

    // Spawn worker threads
    let mut pool = WorkerPool::new();

    for _ in 0..num_threads {
        let work_rx = Arc::clone(&work_rx);
        let result_tx = result_tx.clone();
        let config = Arc::clone(&config);
        let cache = Arc::clone(&cache);
        let template_hash = Arc::clone(&template_hash);
        let shortcode_registry = Arc::clone(&shortcode_registry);
        let progress = Arc::clone(&progress);

        pool.spawn(move || {
            let renderer = Renderer::new();
            let generator = match Generator::new((*config).clone()) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("Failed to create generator: {}", e);
                    return;
                }
            };

            loop {
                let path = {
                    let rx = work_rx.lock().unwrap();
                    rx.recv().ok()
                };

                let path = match path {
                    Some(p) => p,
                    None => break,
                };

                let result = process_post_parallel(
                    &path,
                    &renderer,
                    &generator,
                    &shortcode_registry,
                    &config,
                    &cache,
                    &template_hash,
                    use_cache,
                );

                match &result {
                    BuildResult::Success { .. } => progress.increment_built(),
                    BuildResult::Skipped { .. } => progress.increment_skipped(),
                    BuildResult::Error { .. } => {}
                }

                let _ = result_tx.send(result);
            }
        });
    }

    drop(result_tx);

    // Collect results
    let mut results = Vec::new();
    for result in result_rx {
        results.push(result);
    }

    pool.join().map_err(|e| anyhow::anyhow!(e))?;

    // Update metadata and cache from results
    let mut errors = Vec::new();
    for result in results {
        match result {
            BuildResult::Success {
                path,
                slug,
                category,
                frontmatter,
                file_hash,
                template_hash,
                output_path,
            } => {
                println!("🔨 Built: {}", path.display());
                metadata.upsert_post(slug, category, frontmatter);
                cache.lock().unwrap().update_entry(
                    &path,
                    file_hash,
                    template_hash,
                    output_path,
                );
            }
            BuildResult::Skipped { path, reason } => {
                match reason {
                    SkipReason::Cached => println!("⏭  Skipped (unchanged): {}", path.display()),
                    SkipReason::Draft => println!("   ⚠  Draft - skipping: {}", path.display()),
                }
            }
            BuildResult::Error { path, error } => {
                eprintln!("❌ Error building {}: {}", path.display(), error);
                errors.push((path, error));
            }
        }
    }

    if !errors.is_empty() {
        anyhow::bail!("{} posts failed to build", errors.len());
    }

    // Save cache
    if use_cache {
        cache.lock().unwrap().save()?;
    }
    metadata.save()?;

    // Build pages (sequential for simplicity)
    let pages_dir = Path::new("content/pages");
    if pages_dir.exists() {
        println!("\n📄 Building pages...");
        let renderer = Renderer::new();
        let generator = Generator::new((*config).clone())?;
        let mut pages_built = 0;

        for entry in WalkDir::new(pages_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let path = entry.path();
            println!("🔨 Building page: {}", path.display());

            let mut page = Parser::parse_page_file(path)?;

            if page.frontmatter.draft {
                println!("   ⚠  Draft - skipping output");
                continue;
            }

            let processed_content = shortcode_registry.process(&page.content)?;
            let html = renderer.render_markdown_with_components(
                &processed_content,
                generator.get_tera(),
                &page.slug,
            )?;
            page.rendered_html = Some(html);

            let plugin_ctx = PluginContext {
                config: &config,
                metadata: &metadata,
            };
            let plugin_data = plugin_manager.template_context_page(&page, &plugin_ctx)?;

            let output_path = generator.generate_page(&page, &plugin_data)?;
            println!("   ✓ {}", output_path.display());

            pages_built += 1;
        }

        if pages_built > 0 {
            println!("✅ Built {} page(s)", pages_built);
        }
    }

    // Generate indices with plugin data
    let index_generator = IndexGenerator::new((*config).clone())?;
    index_generator.generate_all(&metadata, &plugin_manager)?;

    // Generate feeds
    println!("📄 Generating RSS feeds...");
    FeedGenerator::generate_all_feeds(
        &config,
        &metadata,
        posts_dir,
        Path::new(&config.build.output_dir),
    )?;

    // Generate search index
    if config.build.search.enabled {
        let search_generator = SearchIndexGenerator::new((*config).clone());
        search_generator.generate(&metadata)?;
    }

    // Generate syntax CSS and copy assets
    println!("🎨 Generating syntax highlighting CSS...");
    let renderer = Renderer::new();
    let generator = Generator::new((*config).clone())?;
    let css_dir = Path::new(&config.build.output_dir).join("css");
    std::fs::create_dir_all(&css_dir)?;
    renderer.write_syntax_css(css_dir.join("syntax.css"))?;

    generator.copy_content_assets()?;
    generator.copy_static_assets()?;

    let elapsed = start_time.elapsed();
    println!("\n✅ Build complete in {:.2}s!", elapsed.as_secs_f64());
    println!("   Built: {}", progress.get_built());
    if use_cache {
        println!("   Skipped: {}", progress.get_skipped());
    }
    println!("   Categories: {}", metadata.get_categories().len());
    println!("   Tags: {}", metadata.get_tags().len());

    Ok(())
}

fn process_post_parallel(
    path: &Path,
    renderer: &Renderer,
    generator: &Generator,
    shortcode_registry: &ShortcodeRegistry,
    _config: &crate::config::SsgConfig,
    cache: &Arc<Mutex<BuildCache>>,
    template_hash: &str,
    use_cache: bool,
) -> BuildResult {
    // Hash file
    let file_hash = match hash_file(path) {
        Ok(h) => h,
        Err(e) => {
            return BuildResult::Error {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        }
    };

    // Check cache
    if use_cache {
        let cache = cache.lock().unwrap();
        if !cache.needs_rebuild(path, &file_hash, template_hash) {
            return BuildResult::Skipped {
                path: path.to_path_buf(),
                reason: SkipReason::Cached,
            };
        }
    }

    // Parse post
    let mut post = match Parser::parse_file(path) {
        Ok(p) => p,
        Err(e) => {
            return BuildResult::Error {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        }
    };

    if post.frontmatter.draft {
        return BuildResult::Skipped {
            path: path.to_path_buf(),
            reason: SkipReason::Draft,
        };
    }

    // Process shortcodes
    let processed_content = match shortcode_registry.process(&post.content) {
        Ok(c) => c,
        Err(e) => {
            return BuildResult::Error {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        }
    };

    // Render markdown
    let base_path = post.category.clone();
    let html = match renderer.render_markdown_with_components(
        &processed_content,
        generator.get_tera(),
        &base_path,
    ) {
        Ok(h) => h,
        Err(e) => {
            return BuildResult::Error {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        }
    };

    post.rendered_html = Some(html);

    // Generate output (without plugin data for now - will add in second pass if needed)
    let plugin_data = std::collections::HashMap::new();
    let output_path = match generator.generate_post(&post, &plugin_data) {
        Ok(p) => p,
        Err(e) => {
            return BuildResult::Error {
                path: path.to_path_buf(),
                error: e.to_string(),
            }
        }
    };

    BuildResult::Success {
        path: path.to_path_buf(),
        slug: post.slug,
        category: post.category,
        frontmatter: post.frontmatter,
        file_hash,
        template_hash: template_hash.to_string(),
        output_path: output_path.to_string_lossy().to_string(),
    }
}

fn build_single_post(post_path: &str) -> Result<()> {
    println!("Building single post: {}\n", post_path);

    let config = load_config()?;
    let renderer = Renderer::new();
    let mut shortcode_registry = ShortcodeRegistry::new();
    let generator = Generator::new(config.clone())?;
    let metadata = MetadataCache::load().unwrap_or_else(|_| MetadataCache::new());

    // Initialize plugin system
    let mut plugin_manager = PluginManager::new();
    plugin_manager.register(Box::new(RelatedPostsPlugin::new()));
    plugin_manager.init_all(&config)?;

    // Register plugin shortcodes
    plugin_manager.register_shortcodes(&mut shortcode_registry);

    let path = Path::new(post_path);

    if !path.exists() {
        anyhow::bail!("Post file not found: {}", post_path);
    }

    let mut post = Parser::parse_file(path)?;

    if post.frontmatter.draft {
        println!("⚠  This is a draft post");
    }

    // Create plugin context
    let plugin_ctx = PluginContext {
        config: &config,
        metadata: &metadata,
    };

    // Plugin hook: after parsing
    plugin_manager.on_post_parsed(&mut post, &plugin_ctx)?;

    // Process shortcodes before markdown rendering
    let processed_content = shortcode_registry.process(&post.content)?;

    let base_path = format!("{}", post.category);
    let mut html = renderer.render_markdown_with_components(
        &processed_content,
        generator.get_tera(),
        &base_path,
    )?;

    // Plugin hook: after rendering
    plugin_manager.on_post_rendered(&mut post, &mut html, &plugin_ctx)?;

    post.rendered_html = Some(html);

    // Collect plugin template data
    let plugin_data = plugin_manager.template_context_post(&post, &plugin_ctx)?;

    let output_path = generator.generate_post(&post, &plugin_data)?;

    println!("\n✅ Built: {}", output_path.display());

    Ok(())
}

fn create_new_post(category: &str, title: &str) -> Result<()> {
    let config = load_config()?;
    let posts_dir = Path::new(&config.build.content_dir);

    let categories = discover_categories(posts_dir)?;

    if !validate_category(category, &categories) {
        println!("⚠️  Category '{}' doesn't exist yet.", category);
        println!();

        if categories.is_empty() {
            println!("No categories found. To create one:");
            println!(
                "  1. Create a directory: mkdir -p {}/{}",
                config.build.content_dir, category
            );
            println!(
                "  2. Optionally add metadata: echo 'name: {}' > {}/{}/.category.yaml",
                category
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .chain(category.chars().skip(1))
                    .collect::<String>(),
                config.build.content_dir,
                category
            );
            println!("  3. Run this command again");
        } else {
            let category_list: Vec<String> = categories
                .iter()
                .map(|c| format!("  - {} ({})", c.slug, c.name))
                .collect();

            println!("Available categories:");
            for cat in category_list {
                println!("{}", cat);
            }
            println!();
            println!("To create a new category:");
            println!(
                "  1. Create a directory: mkdir -p {}/{}",
                config.build.content_dir, category
            );
            println!(
                "  2. Optionally add metadata: echo 'name: Your Name' > {}/{}/.category.yaml",
                config.build.content_dir, category
            );
            println!("  3. Add at least one post to the category");
            println!("  4. Run this command again");
        }

        std::process::exit(0);
    }

    let slug = title
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    let filename = format!("content/posts/{}/{}.md", category, slug);

    if Path::new(&filename).exists() {
        anyhow::bail!("Post already exists: {}", filename);
    }

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

    std::fs::create_dir_all(format!("content/posts/{}", category))?;
    std::fs::write(&filename, content)?;

    println!("✅ Created: {}", filename);
    println!("   Title: {}", title);
    println!("   Category: {}", category);
    println!("   Slug: {}", slug);

    Ok(())
}

fn watch_mode(port: u16) -> Result<()> {
    use notify::{Event, RecursiveMode, Result as NotifyResult, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    println!("🔍 Watch mode starting...");
    println!("   Watching for changes in:");
    println!("   - content/");
    println!("   - themes/");
    println!("   - static/");
    println!("\n   Serving on http://localhost:{}", port);
    println!("   Press Ctrl+C to stop\n");

    // Do initial build
    println!("📦 Initial build...");
    build_all(true)?;
    println!();

    // Start file server in background thread
    let server_thread = std::thread::spawn(move || {
        if let Err(e) = start_dev_server(port) {
            eprintln!("Dev server error: {}", e);
        }
    });

    // Set up file watcher
    let (tx, rx) = channel();

    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<Event>| {
        if let Ok(event) = res {
            tx.send(event).unwrap();
        }
    })?;

    watcher.watch(Path::new("content"), RecursiveMode::Recursive)?;
    watcher.watch(Path::new("themes"), RecursiveMode::Recursive)?;

    if Path::new("static").exists() {
        watcher.watch(Path::new("static"), RecursiveMode::Recursive)?;
    }

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                if !should_rebuild(&event) {
                    continue;
                }

                println!("📝 File changed, rebuilding...");
                match build_all(true) {
                    Ok(_) => println!("✅ Rebuild complete!\n"),
                    Err(e) => eprintln!("❌ Build error: {}\n", e),
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if server_thread.is_finished() {
                    anyhow::bail!("Dev server stopped unexpectedly");
                }
                continue;
            }
            Err(e) => {
                anyhow::bail!("Watch error: {}", e);
            }
        }
    }
}

fn should_rebuild(event: &notify::Event) -> bool {
    use notify::EventKind;

    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
            for path in &event.paths {
                let path_str = path.to_string_lossy();
                if path_str.contains(".build-cache") || path_str.contains("dist/") {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn start_dev_server(port: u16) -> Result<()> {
    use anyhow::Context as _;
    use std::io::Read;
    use std::net::TcpListener;

    let listener =
        TcpListener::bind(format!("127.0.0.1:{}", port)).context("Failed to bind dev server")?;

    println!("🌐 Dev server listening on http://localhost:{}", port);

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Connection error: {}", e);
                continue;
            }
        };

        let mut buffer = [0; 1024];
        if stream.read(&mut buffer).is_err() {
            continue;
        }

        let request = String::from_utf8_lossy(&buffer);
        let request_line = request.lines().next().unwrap_or("");

        let path = if let Some(path_part) = request_line.split_whitespace().nth(1) {
            // Decode URL for filesystem lookup (handles Korean/non-ASCII characters)
            slug::decode_from_url(path_part)
        } else {
            "/".to_string()
        };

        serve_file(&mut stream, &path);
    }

    Ok(())
}

fn serve_file(stream: &mut std::net::TcpStream, path: &str) {
    use std::io::Write;

    let file_path = if path == "/" {
        "dist/index.html".to_string()
    } else if path.ends_with('/') {
        format!("dist{}index.html", path)
    } else {
        format!("dist{}", path)
    };

    let (status, content_type, body) = if let Ok(contents) = std::fs::read(&file_path) {
        let content_type = get_content_type(&file_path);
        ("200 OK", content_type, contents)
    } else {
        let index_path = format!("{}/index.html", file_path);
        if let Ok(contents) = std::fs::read(&index_path) {
            ("200 OK", "text/html", contents)
        } else {
            let body = b"404 Not Found".to_vec();
            ("404 NOT FOUND", "text/plain", body)
        }
    };

    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
        status,
        content_type,
        body.len()
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body);
    let _ = stream.flush();
}

fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}
