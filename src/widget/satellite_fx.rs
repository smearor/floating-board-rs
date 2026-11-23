//! Optional `smearor-commons-fx` integration (feature `fx`, Phase 5.3):
//! snapshot a live widget into a texture, mount an [`FxPlayer`] at its
//! layer position, play an [`EffectSpec`], then swap the live widget
//! back in on `animation-ended`.

use crate::domain::BoardError;
use crate::domain::CardId;
use crate::domain::SatelliteId;
use crate::widget::board::FloatingBoard;
use glib::clone::Downgrade;
use glib::object::Cast;
use glib::subclass::types::ObjectSubclassIsExt;
use gtk4::Widget;
use gtk4::prelude::FixedExt;
use gtk4::prelude::WidgetExt;
use smearor_commons_fx::EffectSpec;
use smearor_commons_fx::gtk::FxPlayOptions;
use smearor_commons_fx::gtk::FxPlayer;
use smearor_commons_fx::gtk::snapshot;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Duration;

impl FloatingBoard {
    /// Plays an [`EffectSpec`] over one satellite of a card: the
    /// satellite's transform stack is snapshotted to a texture, replaced
    /// by an [`FxPlayer`] at the same layer position, and swapped back
    /// in when `animation-ended` fires. The texture is static - geometry
    /// changes during playback do not track; the live widget returns at
    /// its current placement on the next sync.
    ///
    /// `FxPlayOptions::duration == Duration::ZERO` defaults to the
    /// card's `animation_speed_ms` budget.
    pub fn play_satellite_fx(&self, id: &CardId, satellite: &SatelliteId, spec: EffectSpec, options: FxPlayOptions) -> Result<(), BoardError> {
        let cards = self.imp().cards.borrow();
        let entry = cards.get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
        let satellites = entry.satellites.borrow();
        let Some(satellite_entry) = satellites.iter().find(|entry| &entry.id == satellite) else {
            return Err(BoardError::UnknownSatellite {
                card: id.clone(),
                satellite: satellite.clone(),
            });
        };
        let target = satellite_entry.scaling.clone().upcast::<Widget>();
        self.play_widget_fx(&target, spec, options, entry.config.animation_speed_ms)
    }

    /// Same FX swap as [`Self::play_satellite_fx`] for a whole card:
    /// the card's transform stack is snapshotted, replaced by an
    /// [`FxPlayer`], and swapped back in on `animation-ended`.
    /// Satellites keep rendering normally while the effect plays.
    pub fn play_card_fx(&self, id: &CardId, spec: EffectSpec, options: FxPlayOptions) -> Result<(), BoardError> {
        let cards = self.imp().cards.borrow();
        let entry = cards.get(id).ok_or_else(|| BoardError::UnknownCard(id.clone()))?;
        let target = entry.scaling.clone().upcast::<Widget>();
        self.play_widget_fx(&target, spec, options, entry.config.animation_speed_ms)
    }

    /// The snapshot -> [`FxPlayer`] -> swap-back pipeline shared by the
    /// satellite and card FX entry points.
    fn play_widget_fx(&self, target: &Widget, spec: EffectSpec, mut options: FxPlayOptions, speed_ms: u64) -> Result<(), BoardError> {
        let Some(layer) = self.card_layer() else {
            return Err(BoardError::Fx("board has no card layer".to_string()));
        };
        let buffer = snapshot(target).map_err(|error| BoardError::Fx(error.to_string()))?;
        let player = FxPlayer::new();
        player.set_effect(spec, buffer);
        if options.duration.is_zero() {
            options.duration = Duration::from_millis(speed_ms.max(1));
        }
        let position = Vec2::<Pixels>::from(layer.child_position(target));
        let (width, height) = (target.width(), target.height());
        if width > 0 && height > 0 {
            player.set_size_request(width, height);
        }
        layer.put(&player, position.x, position.y);
        let was_visible = target.is_visible();
        target.set_visible(false);
        let target_weak = target.downgrade();
        let layer_weak = layer.downgrade();
        player.connect_animation_ended(move |player| {
            if let Some(layer) = layer_weak.upgrade() {
                layer.remove(player);
            }
            if let Some(target) = target_weak.upgrade() {
                target.set_visible(was_visible);
            }
        });
        player.play(options);
        Ok(())
    }
}
