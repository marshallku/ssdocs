use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::parser::Parser;
use crate::renderer::Renderer;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FeedGenerator;

impl FeedGenerator {
    pub fn generate_rss(
        config: &SsgConfig,
        metadata: &MetadataCache,
        content_dir: &Path,
        output_dir: &Path,
    ) -> Result<()> {
        let recent_posts = metadata.get_recent_posts(10);

        if recent_posts.is_empty() {
            return Ok(());
        }

        let renderer = Renderer::new();
        let last_build_date = chrono::Utc::now().to_rfc2822();

        let mut items = Vec::new();

        for post_meta in recent_posts {
            if post_meta.frontmatter.draft {
                continue;
            }

            let post_path = Self::find_post_file(content_dir, &post_meta.slug)?;
            let post = Parser::parse_file(&post_path)
                .with_context(|| format!("Failed to parse post: {}", post_meta.slug))?;

            let rendered_content = renderer.render_markdown(&post.content);
            let url = format!("{}/{}/{}", config.site.url, post.frontmatter.category, post.slug);

            let category_name = metadata
                .get_category_info()
                .iter()
                .find(|c| c.slug == post.frontmatter.category)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| post.frontmatter.category.clone());

            let tags_xml = if !post.frontmatter.tags.is_empty() {
                post.frontmatter
                    .tags
                    .iter()
                    .map(|tag| format!("        <category><![CDATA[{}]]></category>", tag))
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                String::new()
            };

            let description = post
                .frontmatter
                .description
                .as_deref()
                .unwrap_or(&post.frontmatter.title);

            let pub_date = post.frontmatter.date.to_rfc2822();

            let item = format!(
                r#"    <item>
        <title>{}</title>
        <link>{}</link>
        <dc:creator><![CDATA[{}]]></dc:creator>
        <pubDate>{}</pubDate>
        <category><![CDATA[{}]]></category>{}{}
        <guid isPermaLink="false">{}</guid>
        <description><![CDATA[{}]]></description>
        <content:encoded><![CDATA[{}]]></content:encoded>
    </item>"#,
                Self::escape_xml(&post.frontmatter.title),
                url,
                config.site.author,
                pub_date,
                category_name,
                if tags_xml.is_empty() { "" } else { "\n" },
                tags_xml,
                url,
                Self::escape_xml(description),
                rendered_content
            );

            items.push(item);
        }

        let feed_url = format!("{}/feed.xml", config.site.url);

        let rss_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:content="http://purl.org/rss/1.0/modules/content/" xmlns:wfw="http://wellformedweb.org/CommentAPI/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:sy="http://purl.org/rss/1.0/modules/syndication/" xmlns:slash="http://purl.org/rss/1.0/modules/slash/"
>
<channel>
    <title>{}</title>
    <description>{}</description>
    <language>ko-KR</language>
    <atom:link href="{}" rel="self" type="application/rss+xml" />
    <link>{}</link>
    <lastBuildDate>{}</lastBuildDate>
    <sy:updatePeriod>hourly</sy:updatePeriod>
    <sy:updateFrequency>1</sy:updateFrequency>
{}
</channel>
</rss>
"#,
            Self::escape_xml(&config.site.title),
            Self::escape_xml(&config.site.description),
            feed_url,
            config.site.url,
            last_build_date,
            items.join("\n")
        );

        fs::create_dir_all(output_dir)?;
        let output_path = output_dir.join("feed.xml");
        fs::write(&output_path, rss_xml)?;

        Ok(())
    }

    fn find_post_file(content_dir: &Path, slug: &str) -> Result<PathBuf> {
        let filename = format!("{}.md", slug);

        for entry in WalkDir::new(content_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == filename.as_str() {
                return Ok(entry.path().to_path_buf());
            }
        }

        anyhow::bail!("Post file not found: {}", slug)
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        let input = r#"Hello & <world> "test""#;
        let expected = r#"Hello &amp; &lt;world&gt; &quot;test&quot;"#;
        assert_eq!(FeedGenerator::escape_xml(input), expected);
    }
}
