use wasm_bindgen::prelude::*;
use vertra::text_overlay::TextOverlay as CoreTextOverlay;
use vertra::text_label::{TextLabelHandle, VerticalAlignment};

/// Screen-space text overlay, accessible as `scene.text_overlay`.
///
/// ```js
/// const ov = scene.text_overlay;
///
/// // Load a font (required unless the `default-fonts` feature is active)
/// const bytes = new Uint8Array(await fetch('fonts/Roboto.ttf').then(r => r.arrayBuffer()));
/// ov.load_font('roboto', bytes);
///
/// // Create a label with default font
/// const label = ov.add_label('Score: 0', [20, 20], 24, [1, 1, 1, 1]);
///
/// // Create a label with specific font and alignment
/// const label2 = ov.add_label('HP: 100', [20, 50], 24, [0, 1, 0, 1], 'roboto', null, 'right', 'bottom');
///
/// // Mutate via setters
/// label.text      = 'Score: 100';
/// label.color     = [1, 0, 0, 1];
/// label.position  = [20, 50];
/// label.font_size = 32;
///
/// // Remove
/// label.remove();
/// ```
#[wasm_bindgen]
pub struct TextOverlay {
    #[wasm_bindgen(skip)]
    pub inner: *mut CoreTextOverlay,
}

#[wasm_bindgen]
impl TextOverlay {
    /// Load a TrueType / OpenType font from raw bytes, registered under `font_id`.
    ///
    /// Pass the same `font_id` string to [`TextOverlay::add_label`] to create a
    /// label with this face, or use [`TextLabel::font`] on an existing label to
    /// switch to it. Returns a JS error string on failure (empty id, duplicate
    /// id, or parse error).
    pub fn load_font(&mut self, font_id: &str, font_bytes: &[u8]) -> Result<(), JsValue> {
        unsafe {
            (*self.inner).add_font(font_id, font_bytes)
                .map_err(|e| JsValue::from_str(&e))
        }
    }

    /// Returns the number of fonts currently loaded.
    pub fn font_count(&self) -> usize {
        unsafe { (*self.inner).font_count() }
    }

    /// Returns `true` if a font with `font_id` has been loaded.
    pub fn has_font(&self, font_id: &str) -> bool {
        unsafe { (*self.inner).has_font(font_id) }
    }

    /// Add a screen-space text label.
    ///
    /// * `text`               - String to display.
    /// * `position`           - `[x, y]` pixel position from the top-left corner.
    /// * `font_size`          - Font size in pixels.
    /// * `color`              - `[r, g, b, a]` in `[0.0, 1.0]`.
    /// * `font_id`            - Optional font ID string.
    /// * `zindex`             - Optional draw order. Lower values render first (further back).
    /// * `alignment`          - Optional horizontal anchor: `"left"` (default), `"center"`, or `"right"`.
    /// * `vertical_alignment` - Optional vertical anchor: `"top"` (default), `"middle"`, or `"bottom"`.
    ///
    /// Returns a [`TextLabel`] handle.
    pub fn add_label(
        &mut self,
        text: &str,
        position: Vec<f32>,
        font_size: f32,
        color: Vec<f32>,
        font_id: Option<String>,
        zindex: Option<i32>,
        alignment: Option<String>,
        vertical_alignment: Option<String>,
    ) -> TextLabel {
        let (x, y) = if position.len() >= 2 { (position[0], position[1]) } else { (0.0, 0.0) };
        let c = pad4(&color);
        let align   = parse_alignment(alignment.as_deref());
        let valign  = parse_vertical_alignment(vertical_alignment.as_deref());
        let id = unsafe {
            let mut builder = (*self.inner)
                .add_label(text)
                .at(x, y)
                .with_font_size(font_size)
                .with_color(c)
                .with_alignment(align)
                .with_vertical_alignment(valign);
            if let Some(fid) = font_id {
                builder = builder.with_font(fid);
            }
            if let Some(z) = zindex {
                builder = builder.with_zindex(z);
            }
            builder.build().id
        };
        TextLabel { overlay: self.inner, id }
    }


    /// Remove all text labels.
    pub fn clear(&mut self) {
        unsafe { (*self.inner).clear(); }
    }

    /// Returns the number of active text labels.
    pub fn label_count(&self) -> usize {
        unsafe { (*self.inner).label_count() }
    }
}

/// A handle to a single screen-space text label.
///
/// Returned by [`TextOverlay::add_label`]. To use a specific font, pass the
/// optional `font_id` argument to `add_label`.
/// Use JS property setters to mutate the label in place:
///
/// ```js
/// const label = scene.text_overlay.add_label('Score: 0', [20, 20], 24, [1, 1, 1, 1]);
/// label.text               = 'Score: 100';
/// label.color              = [1, 0, 0, 1];
/// label.position           = [40, 40];
/// label.font_size          = 32;
/// label.visible            = false;
/// label.alignment          = 'center';
/// label.vertical_alignment = 'middle';
/// label.remove();
/// ```
#[wasm_bindgen]
pub struct TextLabel {
    #[wasm_bindgen(skip)]
    pub overlay: *mut CoreTextOverlay,
    #[wasm_bindgen(skip)]
    pub id: usize,
}

#[wasm_bindgen]
impl TextLabel {
    /// The numeric label ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> usize { self.id }

    /// Replace the displayed string.
    #[wasm_bindgen(setter)]
    pub fn set_text(&mut self, text: String) {
        unsafe { TextLabelHandle { id: self.id }.set_text(&mut *self.overlay, text); }
    }

    /// Set RGBA colour as `[r, g, b, a]` in `[0.0, 1.0]`.
    #[wasm_bindgen(setter)]
    pub fn set_color(&mut self, color: Vec<f32>) {
        if color.len() < 4 { return; }
        unsafe {
            TextLabelHandle { id: self.id }
                .set_color(&mut *self.overlay, [color[0], color[1], color[2], color[3]]);
        }
    }

    /// Set pixel position as `[x, y]` from the top-left corner.
    #[wasm_bindgen(setter)]
    pub fn set_position(&mut self, pos: Vec<f32>) {
        if pos.len() < 2 { return; }
        unsafe { TextLabelHandle { id: self.id }.move_to(&mut *self.overlay, pos[0], pos[1]); }
    }

    /// Set font size in pixels.
    #[wasm_bindgen(setter)]
    pub fn set_font_size(&mut self, size: f32) {
        unsafe { TextLabelHandle { id: self.id }.set_font_size(&mut *self.overlay, size); }
    }

    /// Select a font by its string ID (see [`TextOverlay::load_font`]).
    #[wasm_bindgen(setter)]
    pub fn set_font(&mut self, font_id: String) {
        unsafe { TextLabelHandle { id: self.id }.set_font(&mut *self.overlay, font_id); }
    }

    /// Show (`true`) or hide (`false`) the label without removing it.
    #[wasm_bindgen(setter)]
    pub fn set_visible(&mut self, visible: bool) {
        unsafe {
            let handle = TextLabelHandle { id: self.id };
            if visible { handle.show(&mut *self.overlay); } else { handle.hide(&mut *self.overlay); }
        }
    }

    /// Set the draw order.  Lower values render first (further back).
    #[wasm_bindgen(setter)]
    pub fn set_zindex(&mut self, z: i32) {
        unsafe { TextLabelHandle { id: self.id }.set_zindex(&mut *self.overlay, z); }
    }

    /// Set the horizontal alignment / resize anchor.
    /// Accepted values: `"left"` (default), `"center"`, `"right"`, `free`.
    #[wasm_bindgen(setter)]
    pub fn set_alignment(&mut self, alignment: String) {
        let a = parse_alignment(Some(&alignment));
        unsafe { TextLabelHandle { id: self.id }.set_alignment(&mut *self.overlay, a); }
    }

    /// Set the vertical alignment / resize anchor.
    /// Accepted values: `"top"` (default), `"middle"`, `"bottom"`, `free`.
    #[wasm_bindgen(setter)]
    pub fn set_vertical_alignment(&mut self, alignment: String) {
        let a = parse_vertical_alignment(Some(&alignment));
        unsafe { TextLabelHandle { id: self.id }.set_vertical_alignment(&mut *self.overlay, a); }
    }

    /// Remove the label from the overlay.  Returns `true` if it existed.
    pub fn remove(&mut self) -> bool {
        unsafe { TextLabelHandle { id: self.id }.remove(&mut *self.overlay) }
    }
}

fn pad4(v: &[f32]) -> [f32; 4] {
    [
        v.first().copied().unwrap_or(1.0),
        v.get(1).copied().unwrap_or(1.0),
        v.get(2).copied().unwrap_or(1.0),
        v.get(3).copied().unwrap_or(1.0),
    ]
}

fn parse_alignment(s: Option<&str>) -> vertra::text_label::HorizontalAlignment {
    use vertra::text_label::HorizontalAlignment;
    match s {
        Some("center") => HorizontalAlignment::Center,
        Some("right")  => HorizontalAlignment::Right,
        Some("free")   => HorizontalAlignment::Free,
        _              => HorizontalAlignment::Left,
    }
}

fn parse_vertical_alignment(s: Option<&str>) -> vertra::text_label::VerticalAlignment {
    use vertra::text_label::VerticalAlignment;
    match s {
        Some("middle") => VerticalAlignment::Middle,
        Some("bottom") => VerticalAlignment::Bottom,
        Some("free")   => VerticalAlignment::Free,
        _              => VerticalAlignment::Top,
    }
}

