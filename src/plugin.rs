use crate::config::SsgConfig;
use crate::metadata::MetadataCache;
use crate::types::{Page, Post};
use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Context provided to plugins during execution
pub struct PluginContext<'a> {
    #[allow(unused)]
    pub config: &'a SsgConfig,
    pub metadata: &'a MetadataCache,
}

/// Plugin trait for extending ssdocs functionality
pub trait Plugin: Send + Sync {
    /// Plugin name (must be unique)
    fn name(&self) -> &str;

    /// Initialize the plugin with configuration
    fn init(&mut self, _config: &SsgConfig) -> Result<()> {
        Ok(())
    }

    /// Hook: Modify post content after parsing but before rendering
    fn on_post_parsed(&self, _post: &mut Post, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    /// Hook: Modify rendered HTML before template application
    fn on_post_rendered(
        &self,
        _post: &mut Post,
        _html: &mut String,
        _ctx: &PluginContext,
    ) -> Result<()> {
        Ok(())
    }

    /// Hook: Add data to post template context
    fn template_context_post(
        &self,
        _post: &Post,
        _ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        Ok(HashMap::new())
    }

    /// Hook: Add data to page template context
    fn template_context_page(
        &self,
        _page: &Page,
        _ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        Ok(HashMap::new())
    }

    /// Hook: Add data to index template context (homepage, category, tag pages)
    fn template_context_index(&self, _ctx: &PluginContext) -> Result<HashMap<String, JsonValue>> {
        Ok(HashMap::new())
    }
}

/// Plugin manager for loading and executing plugins
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Initialize all plugins
    pub fn init_all(&mut self, config: &SsgConfig) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.init(config)?;
        }
        Ok(())
    }

    /// Execute on_post_parsed hooks
    pub fn on_post_parsed(&self, post: &mut Post, ctx: &PluginContext) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_post_parsed(post, ctx)?;
        }
        Ok(())
    }

    /// Execute on_post_rendered hooks
    pub fn on_post_rendered(
        &self,
        post: &mut Post,
        html: &mut String,
        ctx: &PluginContext,
    ) -> Result<()> {
        for plugin in &self.plugins {
            plugin.on_post_rendered(post, html, ctx)?;
        }
        Ok(())
    }

    /// Collect template context from all plugins for posts
    pub fn template_context_post(
        &self,
        post: &Post,
        ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        let mut context = HashMap::new();

        for plugin in &self.plugins {
            let plugin_context = plugin.template_context_post(post, ctx)?;
            context.extend(plugin_context);
        }

        Ok(context)
    }

    /// Collect template context from all plugins for pages
    pub fn template_context_page(
        &self,
        page: &Page,
        ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        let mut context = HashMap::new();

        for plugin in &self.plugins {
            let plugin_context = plugin.template_context_page(page, ctx)?;
            context.extend(plugin_context);
        }

        Ok(context)
    }

    /// Collect template context from all plugins for index pages
    #[allow(unused)]
    pub fn template_context_index(
        &self,
        ctx: &PluginContext,
    ) -> Result<HashMap<String, JsonValue>> {
        let mut context = HashMap::new();

        for plugin in &self.plugins {
            let plugin_context = plugin.template_context_index(ctx)?;
            context.extend(plugin_context);
        }

        Ok(context)
    }

    /// Get list of registered plugin names
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.iter().map(|p| p.name().to_string()).collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        name: String,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        fn template_context_post(
            &self,
            _post: &Post,
            _ctx: &PluginContext,
        ) -> Result<HashMap<String, JsonValue>> {
            let mut context = HashMap::new();
            context.insert(
                "test_value".to_string(),
                JsonValue::String("test".to_string()),
            );
            Ok(context)
        }
    }

    #[test]
    fn test_plugin_registration() {
        let mut manager = PluginManager::new();
        let plugin = Box::new(TestPlugin {
            name: "test".to_string(),
        });

        manager.register(plugin);
        assert_eq!(manager.list_plugins(), vec!["test"]);
    }
}
