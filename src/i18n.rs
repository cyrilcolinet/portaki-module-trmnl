//! Texte envoyé à la plateforme dans les deux langues à la fois (erreurs, publication), lu dans
//! les bundles du module.

use portaki_sdk::contracts::i18n::I18nText;

/// La clé dans les deux bundles ; la clé elle-même quand elle manque.
pub fn text(key: &str) -> I18nText {
    let read = |raw: &str| -> String {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(raw)
            .ok()
            .and_then(|map| map.get(key).and_then(|v| v.as_str()).map(str::to_string))
            .unwrap_or_else(|| key.to_string())
    };
    I18nText::new(
        read(include_str!("../i18n/fr-FR.json")),
        read(include_str!("../i18n/en-US.json")),
    )
}
