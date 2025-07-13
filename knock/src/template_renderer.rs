use anyhow::Context;
use minijinja::{Environment, Value};
use std::collections::HashMap;

/// A helper struct to translate text
#[derive(Debug)]
pub struct TemplateRenderer {
    templates: Environment<'static>,
}

impl TemplateRenderer {
    pub fn new(language_contents: &str, language: &str) -> anyhow::Result<Self> {
        let mut languages: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(language_contents).context("failed to parse i18n json")?;
        let terms = languages
            .remove(language)
            .with_context(|| format!("unknown language {}", language))?;

        // Note: the suffix ".html" in the template name is important, so that jinja escapes data
        // for HTML
        let mut templates = Environment::new();
        templates.add_template("login.html", include_str!("../web/login.html.j2"))?;
        templates.add_template("portal.html", include_str!("../web/portal.html.j2"))?;
        templates.add_filter("text", move |term: &str| -> String {
            match terms.get(term) {
                Some(text) => text.to_string(),
                None => {
                    tracing::warn!("unknown term: {}", term);
                    term.to_string()
                }
            }
        });

        Ok(Self { templates })
    }

    pub fn render(&self, template_name: &str, data: Value) -> anyhow::Result<String> {
        self.templates
            .get_template(template_name)
            .context("failed to find template")?
            .render(data)
            .context("failed to render template")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minijinja::context;
    use serde_json::json;

    #[test]
    fn test() {
        let language_contents = &json!({"fr": {"one": "un"}}).to_string();
        let mut renderer = TemplateRenderer::new(language_contents, "fr").unwrap();

        renderer
            .templates
            .add_template("test.html", r#"{{ "one" | text }} + {{ two }} = 3"#)
            .unwrap();

        let rendered = renderer
            .render(
                "test.html",
                context! {
                    two => "<deux>"
                },
            )
            .unwrap();

        assert_eq!(rendered, "un + &lt;deux&gt; = 3");
    }
}
