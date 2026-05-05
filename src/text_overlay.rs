//! Screen-space text overlay.
//!
//! [`TextOverlay`] manages a list of [`TextLabel`]s that are rendered as
//! camera-independent HUD text on top of the 3D scene.  Each label is
//! rasterised on the CPU using [`fontdue`] and uploaded as an RGBA texture
//! quad every time its properties change.
//!
//! # Quick start
//! ```rust,ignore
//! scene.text_overlay.add_font("roboto", include_bytes!("Roboto.ttf")).unwrap();
//!
//! let score = scene.text_overlay
//!     .add_label("Score: 0")
//!     .at(20.0, 20.0)
//!     .with_font_size(24.0)
//!     .with_color([1.0, 1.0, 0.0, 1.0])
//!     .with_font("roboto")
//!     .build();
//!
//! // Later:
//! score.set_text(&mut scene.text_overlay, "Score: 100");
//! ```

use crate::mesh::Vertex;
pub use crate::text_label::{TextLabel, TextLabelBuilder, TextLabelHandle};
#[cfg(feature = "default-fonts")]
pub use crate::text_label::DefaultFont;
use crate::text_label::rasterize_text;

/// Manages screen-space [`TextLabel`]s and the fonts used to rasterise them.
///
/// Owned by [`crate::scene::Scene`] and rendered as a final overlay pass in
/// [`crate::scene::Scene::draw_world`].
pub struct TextOverlay {
    pub(crate) labels: Vec<TextLabel>,
    pub(crate) next_id: usize,
    /// Fonts stored as `(font_id, font)` in insertion order.
    pub(crate) fonts: Vec<(String, fontdue::Font)>,
}

impl Default for TextOverlay {
    fn default() -> Self { Self::new() }
}

impl TextOverlay {
    /// Create an empty overlay.
    ///
    /// With the `default-fonts` feature enabled the bundled fonts are loaded
    /// automatically if their placeholder files contain valid TTF data.
    pub fn new() -> Self {
        #[allow(unused_mut)]
        let mut this = Self { labels: Vec::new(), next_id: 0, fonts: Vec::new() };
        #[cfg(feature = "default-fonts")]
        this.load_default_fonts();
        this
    }

    /// Parse `font_bytes` as a TrueType / OpenType font and register it under
    /// the given string `font_id`.
    ///
    /// The `font_id` is used in [`TextLabelBuilder::with_font`] and
    /// [`TextLabelHandle::set_font`] to select this face.  An empty `font_id`
    /// is not allowed, use any non-empty string (e.g. `"roboto"`, `"sans"`).
    ///
    /// # Errors
    /// Returns an error string when:
    /// * `font_id` is empty.
    /// * A font with the same `font_id` is already registered.
    /// * `fontdue` cannot parse the bytes.
    pub fn add_font(
        &mut self,
        font_id: impl Into<String>,
        font_bytes: &[u8],
    ) -> Result<(), String> {
        let id = font_id.into();
        if id.is_empty() {
            return Err("TextOverlay: font_id must not be empty".to_string());
        }
        if self.fonts.iter().any(|(k, _)| k == &id) {
            return Err(format!("TextOverlay: font id '{id}' is already registered"));
        }
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("TextOverlay: failed to load font '{id}': {e}"))?;
        self.fonts.push((id, font));
        Ok(())
    }

    /// Returns the number of fonts currently loaded.
    pub fn font_count(&self) -> usize { self.fonts.len() }

    /// Returns `true` if a font with the given ID has been loaded.
    pub fn has_font(&self, font_id: &str) -> bool {
        self.fonts.iter().any(|(k, _)| k == font_id)
    }

    /// Load the bundled default font stack.
    ///
    /// Registers fonts under the IDs `"sans"`, `"serif"`, and `"mono"` (see
    /// [`DefaultFont`]).  Called automatically by [`TextOverlay::new`] when
    /// the `default-fonts` feature is enabled.  Empty placeholder files are
    /// silently skipped; already-registered IDs are also skipped.
    #[cfg(feature = "default-fonts")]
    pub fn load_default_fonts(&mut self) {
        const SANS_BYTES:  &[u8] = include_bytes!("fonts/sans.ttf");
        const SERIF_BYTES: &[u8] = include_bytes!("fonts/serif.ttf");
        const MONO_BYTES:  &[u8] = include_bytes!("fonts/mono.ttf");

        for (name, bytes) in [("sans", SANS_BYTES), ("serif", SERIF_BYTES), ("mono", MONO_BYTES)] {
            if bytes.is_empty() {
                eprintln!(
                    "[vertra] default-fonts: `src/fonts/{name}.ttf` is empty, \
                     the downloaded binary might be corrupted."
                );
                continue;
            }
            if self.has_font(name) { continue; }
            if let Err(e) = self.add_font(name, bytes) {
                eprintln!("[vertra] default-fonts: failed to parse `src/fonts/{name}.ttf`: {e}");
            }
        }
    }

    /// Begin building a new label with the given text.
    ///
    /// Chain builder methods to configure position, colour, font, etc., then
    /// call [`TextLabelBuilder::build`] to insert the label and receive a
    /// [`TextLabelHandle`].
    ///
    /// ```rust,ignore
    /// let hp = scene.text_overlay
    ///     .add_label("HP: 100")
    ///     .at(10.0, 10.0)
    ///     .with_font("roboto")
    ///     .with_color([0.0, 1.0, 0.0, 1.0])
    ///     .build();
    /// ```
    pub fn add_label(&mut self, text: impl Into<String>) -> TextLabelBuilder<'_> {
        TextLabelBuilder {
            overlay:   self,
            text:      text.into(),
            x:         0.0,
            y:         0.0,
            font_size: 16.0,
            color:     [1.0, 1.0, 1.0, 1.0],
            font_id:   String::new(), // empty = first loaded font
            visible:   true,
        }
    }

    /// Returns a slice of all live labels.
    pub fn labels(&self) -> &[TextLabel] { &self.labels }

    /// Returns the number of live labels.
    pub fn label_count(&self) -> usize { self.labels.len() }

    /// Remove all labels.
    pub fn clear(&mut self) { self.labels.clear(); }

    /// Look up a font by string ID.  An empty ID resolves to the first font.
    fn get_font(&self, font_id: &str) -> Option<&fontdue::Font> {
        if font_id.is_empty() {
            self.fonts.first().map(|(_, f)| f)
        } else {
            self.fonts.iter().find(|(k, _)| k == font_id).map(|(_, f)| f)
        }
    }

    /// Rasterise `label` to an RGBA8 bitmap `(pixels, width, height)`.
    /// Returns `None` when no fonts are loaded or the font ID is not found.
    pub(crate) fn rasterize_label(&self, label: &TextLabel) -> Option<(Vec<u8>, u32, u32)> {
        if self.fonts.is_empty() { return None; }
        let font = self.get_font(&label.font_id)?;
        Some(rasterize_text(font, &label.text, label.font_size, label.color))
    }

    /// Build a screen-space quad at pixel `(x, y)` with size `(w, h)`.
    pub(crate) fn build_quad(x: f32, y: f32, w: f32, h: f32) -> (Vec<Vertex>, Vec<u32>) {
        let verts = vec![
            Vertex { position: [x,     y,     0.0], color: [1.0; 3], uv: [0.0, 0.0] },
            Vertex { position: [x + w, y,     0.0], color: [1.0; 3], uv: [1.0, 0.0] },
            Vertex { position: [x + w, y + h, 0.0], color: [1.0; 3], uv: [1.0, 1.0] },
            Vertex { position: [x,     y + h, 0.0], color: [1.0; 3], uv: [0.0, 1.0] },
        ];
        (verts, vec![0u32, 1, 2, 0, 2, 3])
    }
}


