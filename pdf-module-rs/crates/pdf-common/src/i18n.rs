//! Internationalization (i18n) support via Fluent.
//!
//! Loads translation files from `locales/` directory and provides
//! runtime language selection based on Accept-Language header or
//! user preference.
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::i18n::Translator;
//!
//! let t = Translator::load(&["en-US", "zh-CN"])?;
//! let greeting = t.get("hello-world", Some(&[("name", "World")]))?;
//! ```

use fluent::{FluentResource, bundle::FluentBundle};
use intl_memoizer::concurrent::IntlLangMemoizer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use unic_langid::LanguageIdentifier;

pub struct Translator {
    bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource, IntlLangMemoizer>>,
    default_lang: LanguageIdentifier,
    supported_langs: Vec<LanguageIdentifier>,
}

impl Translator {
    /// Load all `.ftl` files from `locales/` directory.
    ///
    /// Directory structure:
    /// ```text
    /// locales/
    ///   en-US/
    ///     main.ftl
    ///   zh-CN/
    ///     main.ftl
    /// ```
    pub fn load(locales_dir: impl AsRef<Path>, default_lang: &str) -> Result<Self, String> {
        let default_lang: LanguageIdentifier = default_lang
            .parse()
            .map_err(|e: unic_langid::LanguageIdentifierError| e.to_string())?;
        let mut bundles = HashMap::new();
        let mut supported_langs = Vec::new();

        let dir = fs::read_dir(locales_dir.as_ref()).map_err(|e| e.to_string())?;
        for entry in dir {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                continue;
            }

            let lang_str = entry.file_name().to_string_lossy().to_string();
            let lang_id: LanguageIdentifier = lang_str
                .parse()
                .map_err(|e: unic_langid::LanguageIdentifierError| e.to_string())?;

            let mut bundle = FluentBundle::new_concurrent(vec![lang_id.clone()]);

            let lang_dir = entry.path();
            for ftl_entry in fs::read_dir(&lang_dir).map_err(|e| e.to_string())? {
                let ftl_entry = ftl_entry.map_err(|e| e.to_string())?;
                let path = ftl_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                    continue;
                }

                let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                let resource =
                    FluentResource::try_new(content).map_err(|(_, e)| format!("{:?}", e))?;
                bundle.add_resource(resource).map_err(|e| format!("{:?}", e))?;
            }

            bundles.insert(lang_id.clone(), bundle);
            supported_langs.push(lang_id);
        }

        if bundles.is_empty() {
            return Err("No locale directories found".into());
        }

        Ok(Self { bundles, default_lang, supported_langs })
    }

    /// Get translated message with optional arguments.
    pub fn get(&self, message_id: &str, args: Option<&[(&str, &str)]>) -> Result<String, String> {
        self.get_with_lang(message_id, args, &self.default_lang)
    }

    /// Get translated message in a specific language.
    pub fn get_with_lang(
        &self,
        message_id: &str,
        args: Option<&[(&str, &str)]>,
        lang: &LanguageIdentifier,
    ) -> Result<String, String> {
        let bundle = self
            .bundles
            .get(lang)
            .or_else(|| self.bundles.get(&self.default_lang))
            .ok_or_else(|| format!("Language {:?} not loaded", lang))?;

        let msg = bundle
            .get_message(message_id)
            .ok_or_else(|| format!("Message '{}' not found", message_id))?;

        let pattern =
            msg.value().ok_or_else(|| format!("Message '{}' has no value", message_id))?;

        let mut errors = Vec::new();
        let mut fluent_args = fluent::FluentArgs::new();
        if let Some(args) = args {
            for (key, value) in args {
                fluent_args.set(*key, fluent::FluentValue::from(*value));
            }
        }

        let result = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
        if !errors.is_empty() {
            return Err(format!("Formatting errors: {:?}", errors));
        }

        Ok(result.into_owned())
    }

    pub fn supported_languages(&self) -> &[LanguageIdentifier] {
        &self.supported_langs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn translator_loads_and_translates() {
        let tmp = tempfile::tempdir().unwrap();
        let en_dir = tmp.path().join("en-US");
        fs::create_dir_all(&en_dir).unwrap();
        fs::write(en_dir.join("main.ftl"), "hello-world = Hello, { $name }!\n").unwrap();

        let t = Translator::load(tmp.path(), "en-US").unwrap();
        let msg = t.get("hello-world", Some(&[("name", "World")])).unwrap();
        // Fluent wraps interpolated values in Unicode bidi isolation markers.
        // Strip them for comparison.
        let stripped: String =
            msg.chars().filter(|c| !matches!(c, '\u{2068}' | '\u{2069}')).collect();
        assert_eq!(stripped, "Hello, World!");
    }

    #[test]
    fn missing_message_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let en_dir = tmp.path().join("en-US");
        fs::create_dir_all(&en_dir).unwrap();
        fs::write(en_dir.join("main.ftl"), "greeting = Hi\n").unwrap();

        let t = Translator::load(tmp.path(), "en-US").unwrap();
        assert!(t.get("non-existent", None).is_err());
    }
}
