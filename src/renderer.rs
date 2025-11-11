use anyhow::Result;
use pulldown_cmark::{html, Options, Parser as MdParser};
use syntect::highlighting::{ThemeSet, Theme};
use syntect::parsing::SyntaxSet;
use syntect::html::highlighted_html_for_string;

pub struct Renderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Render markdown to HTML
    pub fn render_markdown(&self, markdown: &str) -> String {
        let options = Options::all();
        let parser = MdParser::new_ext(markdown, options);

        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        html_output
    }

    /// Highlight code with Syntect
    /// This is currently unused but available for future enhancement
    #[allow(dead_code)]
    pub fn highlight_code(&self, code: &str, lang: &str) -> Result<String> {
        let syntax = self.syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self.get_theme();

        Ok(highlighted_html_for_string(code, &self.syntax_set, syntax, theme)?)
    }

    fn get_theme(&self) -> &Theme {
        &self.theme_set.themes["base16-ocean.dark"]
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let renderer = Renderer::new();
        let md = "# Hello\n\nThis is **bold**.";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_render_markdown_with_code() {
        let renderer = Renderer::new();
        let md = "```rust\nfn main() {}\n```";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn test_render_markdown_with_links() {
        let renderer = Renderer::new();
        let md = "[Click here](https://example.com)";
        let html = renderer.render_markdown(md);

        assert!(html.contains("<a href=\"https://example.com\">"));
        assert!(html.contains("Click here"));
    }
}
