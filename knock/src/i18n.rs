use anyhow::{Context, bail};
use std::collections::HashMap;

/// A helper struct to translate text
#[derive(Debug)]
pub struct I18n {
    languages: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug)]
pub struct Translator<'a>(&'a HashMap<String, String>);

impl I18n {
    pub fn new(source: &str) -> anyhow::Result<Self> {
        let languages = serde_json::from_str(source).context("failed to parse i18n json")?;
        Ok(Self { languages })
    }

    pub fn translator(&self, lang: &str) -> anyhow::Result<Translator> {
        Ok(Translator(
            self.languages
                .get(lang)
                .with_context(|| format!("unknown language: {}", lang))?,
        ))
    }
}

impl<'a> Translator<'a> {
    pub fn translate(&self, key: &'a str) -> &'a str {
        match self.0.get(key) {
            Some(term) => term,
            None => {
                tracing::warn!("unknown term: {}", key);
                key
            }
        }
    }

    pub fn translate_placeholders(&self, text: &str) -> anyhow::Result<String> {
        let mut translated = String::with_capacity(text.len());
        let mut rest = text;

        loop {
            match rest.split_once("[[") {
                None => {
                    translated += rest;
                    break;
                }
                Some((before, after)) => {
                    translated += before;
                    match after.split_once("]]") {
                        None => {
                            bail!("unmatched [[");
                        }
                        Some((key, new_rest)) => {
                            translated += self.translate(key);
                            rest = new_rest;
                        }
                    }
                }
            }
        }

        Ok(translated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test() {
        let i18n = I18n::new(&json!({"fr": {"one": "un", "two": "deux"}}).to_string()).unwrap();
        let translated = i18n
            .translator("fr")
            .unwrap()
            .translate_placeholders("1 is [[one]], 2 is [[two]], both make 3")
            .unwrap();
        assert_eq!(translated, "1 is un, 2 is deux, both make 3");
    }
}
