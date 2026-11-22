/// Which renderer draws a card's chrome elements.
///
/// `Builtin` renders the stock GTK4/CSS chrome (default look of
/// `floating-board`). `Adwaita` renders libadwaita chrome (`adw::HeaderBar`,
/// `adw::WindowTitle`, `gtk4::WindowControls`) and is only available with
/// the crate's `adwaita` feature; without it the card falls back to
/// `Builtin` with a warning. Adwaita also requires the consumer to call
/// `adw::init()` (or run an `adw::Application`) before cards are built.
///
/// A configured non-default `CardPalette` overrides the Adwaita theme
/// colors through scoped CSS - the palette wins over the theme, the
/// widget structure stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum ChromeStyle {
    /// The stock floating-board chrome (GTK4 widgets + scoped CSS).
    #[default]
    Builtin,
    /// libadwaita chrome (feature `adwaita`; needs `adw::init()` in the
    /// consumer application).
    Adwaita,
}

impl ChromeStyle {
    /// The effective style after feature availability: `Adwaita`
    /// resolves to `Builtin` when the crate's `adwaita` feature is
    /// disabled (compile-time constant, no runtime probe).
    pub fn resolved(self) -> Self {
        match self {
            Self::Builtin => self,
            Self::Adwaita if cfg!(feature = "adwaita") => self,
            Self::Adwaita => Self::Builtin,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_builtin() {
        assert_eq!(ChromeStyle::default(), ChromeStyle::Builtin);
    }

    #[test]
    fn resolved_falls_back_only_for_adwaita() {
        assert_eq!(ChromeStyle::Builtin.resolved(), ChromeStyle::Builtin);
        #[cfg(feature = "adwaita")]
        assert_eq!(ChromeStyle::Adwaita.resolved(), ChromeStyle::Adwaita);
        #[cfg(not(feature = "adwaita"))]
        assert_eq!(ChromeStyle::Adwaita.resolved(), ChromeStyle::Builtin);
    }
}
