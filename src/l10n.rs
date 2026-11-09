use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
use std::sync::OnceLock;
use unic_langid::langid;

static L10N: OnceLock<L10n> = OnceLock::new();

/// Localization for the few user-facing strings owned by this crate.
///
/// The locale is resolved once from `LANGUAGE`/`LANG`; all card content is
/// owned by the consumer and is not localized here.
pub struct L10n {
    bundle: FluentBundle<FluentResource>,
}

impl L10n {
    fn new() -> Self {
        let german = std::env::var("LANGUAGE")
            .or_else(|_| std::env::var("LANG"))
            .map(|value| value.starts_with("de"))
            .unwrap_or(false);
        let source = if german {
            include_str!("../i18n/de/main.ftl")
        } else {
            include_str!("../i18n/en/main.ftl")
        };
        let locale = if german { langid!("de") } else { langid!("en") };
        let mut bundle = FluentBundle::new_concurrent(vec![locale]);
        if let Ok(resource) = FluentResource::try_new(source.to_string()) {
            let _ = bundle.add_resource(resource);
        }
        Self { bundle }
    }

    /// Returns the localized message for `id`, or `id` itself as fallback.
    pub fn message(&self, id: &str) -> String {
        let mut errors = Vec::new();
        self.bundle
            .get_message(id)
            .and_then(|message| message.value())
            .map(|pattern| self.bundle.format_pattern(pattern, None, &mut errors).into_owned())
            .unwrap_or_else(|| id.to_string())
    }
}

/// Returns the process-wide localization instance.
pub(crate) fn l10n() -> &'static L10n {
    L10N.get_or_init(L10n::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_message_resolves() {
        let message = l10n().message("card-close");
        assert!(!message.is_empty());
        assert_ne!(message, "card-close");
    }

    #[test]
    fn unknown_message_falls_back_to_id() {
        assert_eq!(l10n().message("no-such-message"), "no-such-message");
    }
}
