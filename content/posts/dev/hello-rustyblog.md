---
title: "Hello RustyBlog!"
date: 2025-11-11T10:00:00Z
category: dev
tags: [rust, ssg, webdev]
description: "My first post built with the new Rust-powered static site generator"
draft: false
---

# Welcome to RustyBlog

This is the first post built with our **custom Rust static site generator**!

## Why Build This?

We moved away from Next.js because:

- **Memory overhead**: 200MB+ for serving static files
- **Slow builds**: Rebuilding all posts for a single change
- **No incremental builds**: Can't publish just one new post
- **Vendor lock-in**: Too coupled to Vercel

## What We Gained

With RustyBlog:

- **~10MB runtime memory** (just static file serving)
- **Fast builds** with intelligent caching
- **Incremental publishing** - add one post, rebuild in <1 second
- **Full control** over the entire pipeline

![Example Image](./test-image.png)

## Features

### Markdown Support

All standard markdown features work:

- **Bold text**
- _Italic text_
- `inline code`
- [Links](https://rust-lang.org)

### Code Blocks

```rust
fn main() {
    println!("Hello, RustyBlog!");
}
```

```javascript
function greet() {
  console.log("JavaScript works too!");
}
```

### Lists

Unordered:

- First item
- Second item
- Third item

Ordered:

1. Step one
2. Step two
3. Step three

### Blockquotes

> This is a blockquote.
> It can span multiple lines.

## What's Next?

Upcoming features:

1. Index generation (homepage, category pages)
2. Tag pages
3. Pagination
4. RSS feeds
5. Search functionality

Stay tuned!
