use crate::plugin::{Plugin, PluginContext};
use crate::types::Post;
use anyhow::Result;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

/// Plugin that adds related posts to the template context
pub struct RelatedPostsPlugin {
    limit: usize,
}

impl RelatedPostsPlugin {
    pub fn new() -> Self {
        Self { limit: 3 }
    }
}

impl Plugin for RelatedPostsPlugin {
    fn name(&self) -> &str {
        "related_posts"
    }

    fn template_context_post(
        &self,
        post: &Post,
        ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        let mut context = HashMap::new();

        let mut posts: Vec<_> = ctx
            .metadata
            .posts
            .iter()
            .filter(|p| p.category == post.category && p.slug != post.slug)
            .collect();

        posts.sort_by(|a, b| b.frontmatter.date.cmp(&a.frontmatter.date));

        let related_posts: Vec<_> = posts.into_iter().take(self.limit).collect();
        let related_json = json!(related_posts);
        context.insert("related_posts".to_string(), related_json);

        Ok(context)
    }
}

impl Default for RelatedPostsPlugin {
    fn default() -> Self {
        Self::new()
    }
}
