# ssdocs - Static Site Generator

A blazing-fast, memory-efficient static site generator built in Rust for the marshallku blog.

## Quick Start

### Build the project

```bash
cargo build --release
```

### Create a new post

```bash
cargo run -- new dev "My First Post"
```

### Build your site

```bash
# Full build
cargo run -- build

# Incremental build (uses cache)
cargo run -- build --incremental

# Build specific post
cargo run -- build --post content/posts/dev/my-post.md
```

### View your site

The generated files are in `dist/`. You can serve them with any static file server:

```bash
# Using Python
python3 -m http.server 8000 --directory dist

# Using a simple Rust server (if you have it installed)
miniserve dist
```

## Project Structure

```
ssdocs/
├── src/                    # Rust source code
│   ├── main.rs            # CLI and build logic
│   ├── types.rs           # Core types (Post, Config, etc.)
│   ├── parser.rs          # Markdown + frontmatter parsing
│   ├── renderer.rs        # Markdown → HTML rendering
│   ├── generator.rs       # Template application
│   └── cache.rs           # Build cache management
├── content/
│   └── posts/             # Your blog posts
│       ├── dev/
│       ├── chat/
│       ├── gallery/
│       └── notice/
├── templates/             # Tera templates
│   ├── base.html
│   └── post.html
├── static/                # Static assets
│   └── css/
│       └── main.css
└── dist/                  # Build output (gitignored)
```

## Commands

### `ssg build`

Build all posts in `content/posts/`.

Options:
- `--incremental`, `-i` - Use cache to skip unchanged files
- `--post <path>`, `-p <path>` - Build only a specific post

### `ssg new`

Create a new blog post with pre-filled frontmatter.

```bash
ssg new <category> "<title>"
```

Example:
```bash
ssg new dev "Building a Rust SSG"
# Creates: content/posts/dev/building-a-rust-ssg.md
```

### `ssg watch` (Coming Soon)

Watch for file changes and automatically rebuild.

## Documentation

For detailed documentation, see:

- [ARCHITECTURE.md](../rustyblog/ARCHITECTURE.md) - System architecture
- [IMPLEMENTATION_PLAN.md](../rustyblog/IMPLEMENTATION_PLAN.md) - Build guide
- [REFERENCE.md](../rustyblog/REFERENCE.md) - CLI and template reference
- [INDEX_AND_TEMPLATES.md](../rustyblog/INDEX_AND_TEMPLATES.md) - Deep dive on indices and templates

## License

Custom software for marshallku blog.
