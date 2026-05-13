//! Individual screen-space text label representation.

/// Well-known font string IDs loaded by the `default-fonts` feature.
///
/// | Variant | String ID | File                  | Font         |
/// |---------|-----------|-----------------------|--------------|
/// | `Sans`  | `"sans"`  | `src/fonts/sans.ttf`  | Google Sans  |
/// | `Serif` | `"serif"` | `src/fonts/serif.ttf` | Roboto Serif |
/// | `Mono`  | `"mono"`  | `src/fonts/mono.ttf`  | Roboto Mono  |
///
/// Use [`DefaultFont::id`] to obtain the string ID, e.g.:
/// ```rust,ignore
/// builder.with_font(DefaultFont::Sans.id())
/// ```
#[cfg(feature = "default-fonts")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultFont {
    /// The sans-serif face (string id `"sans"`).
    Sans,
    /// The serif face (string id `"serif"`).
    Serif,
    /// The monospace face (string id `"mono"`).
    Mono,
}

#[cfg(feature = "default-fonts")]
impl DefaultFont {
    /// Returns the string font ID for this variant.
    pub fn id(self) -> &'static str {
        match self {
            Self::Sans  => "sans",
            Self::Serif => "serif",
            Self::Mono  => "mono",
        }
    }
}

/// A single screen-space text label.
///
/// Positions are in **pixel coordinates** from the top-left corner of the
/// viewport.  Obtain one via [`crate::text_overlay::TextOverlay::add_label`] + [`TextLabelBuilder::build`]
/// and read it back through [`TextLabelHandle::label`].
#[derive(Debug, Clone)]
pub struct TextLabel {
    /// Unique identifier.
    pub id: usize,
    /// The string to display.
    pub text: String,
    /// Horizontal pixel position from the left edge.
    pub x: f32,
    /// Vertical pixel position from the top edge.
    pub y: f32,
    /// Font size in pixels.
    pub font_size: f32,
    /// RGBA colour in `[0.0, 1.0]` linear space.
    pub color: [f32; 4],
    /// Whether this label is rendered this frame.
    pub visible: bool,
    /// String ID of the font to use (see [`crate::text_overlay::TextOverlay::add_font`]).
    /// An empty string means "use the first loaded font".
    pub font_id: String,
    /// Drawing order: lower values are rendered first (further back).
    /// Defaults to the insertion order index so that labels added later
    /// appear on top.
    pub zindex: i32,
    /// Set whenever a property changes so the GPU texture is re-uploaded.
    pub dirty: bool,
    /// Set when only the position (x/y) changed — rebuilds the vertex buffer
    /// but skips the expensive CPU rasterization and GPU texture re-upload.
    pub position_dirty: bool,
    /// Actual pixel width of the last rasterized bitmap (0 until first render).
    pub rasterized_w: u32,
    /// Actual pixel height of the last rasterized bitmap (0 until first render).
    pub rasterized_h: u32,
    /// The `font_size` value that was in effect when the bitmap was last
    /// rasterised.  Used to scale the on-screen quad during a resize drag
    /// without re-rasterising (draft mode).  0.0 means "not yet rasterised".
    pub rasterized_font_size: f32,
}

/// Fluent builder for creating a new [`TextLabel`].
///
/// Returned by [`crate::text_overlay::TextOverlay::add_label`].
/// Call [`build`](Self::build) to insert the label and receive a
/// [`TextLabelHandle`].
pub struct TextLabelBuilder<'a> {
    pub(crate) overlay:   &'a mut crate::text_overlay::TextOverlay,
    pub(crate) text:      String,
    pub(crate) x:         f32,
    pub(crate) y:         f32,
    pub(crate) font_size: f32,
    pub(crate) color:     [f32; 4],
    pub(crate) font_id:   String,
    pub(crate) visible:   bool,
    pub(crate) zindex:    Option<i32>,
}

impl<'a> TextLabelBuilder<'a> {
    /// Set the pixel position `(x, y)` from the top-left corner.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x; self.y = y; self
    }

    /// Set the RGBA colour `[r, g, b, a]` in `[0.0, 1.0]`.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color; self
    }

    /// Set the font size in pixels.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size; self
    }

    /// Choose a font by its string ID (see [`crate::text_overlay::TextOverlay::add_font`]).
    ///
    /// An empty string or no call to `with_font` uses the first loaded font.
    pub fn with_font(mut self, font_id: impl Into<String>) -> Self {
        self.font_id = font_id.into(); self
    }

    /// Override the draw order.  Lower values render first (further back).
    /// When not called, defaults to insertion order.
    pub fn with_zindex(mut self, z: i32) -> Self {
        self.zindex = Some(z); self
    }

    /// Start the label hidden; call [`TextLabelHandle::show`] to reveal it.
    pub fn hidden(mut self) -> Self {
        self.visible = false; self
    }

    /// Insert the label into the overlay and return a handle.
    ///
    /// The mutable borrow of the overlay ends here, so you can use the scene
    /// freely afterwards.
    pub fn build(self) -> TextLabelHandle {
        let id = self.overlay.next_id;
        self.overlay.next_id += 1;
        let zindex = self.zindex.unwrap_or(id as i32);
        self.overlay.labels.insert(id, TextLabel {
            id,
            text:      self.text,
            x:         self.x,
            y:         self.y,
            font_size: self.font_size,
            color:     self.color,
            visible:   self.visible,
            font_id:        self.font_id,
            zindex,
            dirty:          true,
            position_dirty: false,
            rasterized_w:   0,
            rasterized_h:   0,
            rasterized_font_size: 0.0,
        });
        TextLabelHandle { id }
    }
}

/// A lightweight, copyable handle to a text label.
///
/// All mutation methods take `&mut TextOverlay` explicitly, which keeps the
/// handle lifetime-independent and allows it to be stored anywhere.
///
/// ```rust,ignore
/// let score = overlay.add_label("0").at(20.0, 20.0).build();
///
/// score.set_text(&mut overlay, "42");
/// score.set_font(&mut overlay, "mono");
/// score.remove(&mut overlay);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextLabelHandle {
    /// The numeric label ID.  You can construct a handle from a bare ID with
    /// `TextLabelHandle { id }` when crossing FFI boundaries.
    pub id: usize,
}

impl TextLabelHandle {
    /// Borrow the underlying [`TextLabel`], or `None` if already removed.
    pub fn label<'a>(&self, overlay: &'a crate::text_overlay::TextOverlay) -> Option<&'a TextLabel> {
        overlay.labels.get(&self.id)
    }

    /// Returns `true` if the label still exists in `overlay`.
    pub fn exists(&self, overlay: &crate::text_overlay::TextOverlay) -> bool {
        overlay.labels.contains_key(&self.id)
    }

    /// Replace the displayed text.  Returns `false` if the label was removed.
    pub fn set_text(&self, overlay: &mut crate::text_overlay::TextOverlay, text: impl Into<String>) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.text  = text.into();
            l.dirty = true;
            true
        } else { false }
    }

    /// Move to a new pixel position.  Returns `false` if already removed.
    pub fn move_to(&self, overlay: &mut crate::text_overlay::TextOverlay, x: f32, y: f32) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.x = x; l.y = y; l.position_dirty = true; true
        } else { false }
    }

    /// Change the RGBA colour.  Returns `false` if already removed.
    pub fn set_color(&self, overlay: &mut crate::text_overlay::TextOverlay, color: [f32; 4]) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.color = color; l.dirty = true; true
        } else { false }
    }

    /// Change the font size in pixels.  Returns `false` if already removed.
    pub fn set_font_size(&self, overlay: &mut crate::text_overlay::TextOverlay, size: f32) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.font_size = size; l.dirty = true; true
        } else { false }
    }

    /// Switch to a different font by string ID.  Returns `false` if removed.
    pub fn set_font(&self, overlay: &mut crate::text_overlay::TextOverlay, font_id: impl Into<String>) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.font_id = font_id.into(); l.dirty = true; true
        } else { false }
    }

    /// Override the draw order.  Lower values render first (further back).
    /// Returns `false` if already removed.
    pub fn set_zindex(&self, overlay: &mut crate::text_overlay::TextOverlay, z: i32) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.zindex = z; true
        } else { false }
    }

    /// Make the label visible.  Returns `false` if already removed.
    pub fn show(&self, overlay: &mut crate::text_overlay::TextOverlay) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.visible = true; true
        } else { false }
    }

    /// Hide the label without removing it.  Returns `false` if already removed.
    pub fn hide(&self, overlay: &mut crate::text_overlay::TextOverlay) -> bool {
        if let Some(l) = overlay.labels.get_mut(&self.id) {
            l.visible = false; true
        } else { false }
    }

    /// Remove the label from `overlay`.  Returns `false` if already removed.
    pub fn remove(&self, overlay: &mut crate::text_overlay::TextOverlay) -> bool {
        overlay.labels.remove(&self.id).is_some()
    }
}

pub(crate) fn rasterize_text(
    font: &fontdue::Font,
    text: &str,
    font_size: f32,
    color: [f32; 4],
) -> (Vec<u8>, u32, u32) {
    if text.is_empty() {
        return (vec![0u8; 4], 1, 1);
    }

    let mut glyphs: Vec<(fontdue::Metrics, Vec<u8>)> = Vec::with_capacity(text.len());
    let mut total_advance       = 0.0f32;
    let mut max_above_baseline: i32 = 0;
    let mut min_below_baseline: i32 = 0;

    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, font_size);
        total_advance += metrics.advance_width;
        let above = metrics.ymin + metrics.height as i32;
        if above         > max_above_baseline { max_above_baseline = above; }
        if metrics.ymin  < min_below_baseline { min_below_baseline = metrics.ymin; }
        glyphs.push((metrics, bitmap));
    }

    let text_height = ((max_above_baseline - min_below_baseline) as u32).max(1) + 2;
    // Use ceil so fractional sub-pixel advances don't clip the last glyph.
    let text_width  = (total_advance.ceil() as u32).max(1) + 2;

    let mut pixels   = vec![0u8; (text_width * text_height * 4) as usize];
    let r            = (color[0] * 255.0) as u8;
    let g            = (color[1] * 255.0) as u8;
    let b            = (color[2] * 255.0) as u8;
    let base_alpha   = color[3];
    let baseline_y   = max_above_baseline as i32 + 1;
    // Use a float cursor to accumulate sub-pixel advances accurately.
    let mut cursor_x = 1.0f32;

    for (metrics, bitmap) in &glyphs {
        for gy in 0..metrics.height {
            for gx in 0..metrics.width {
                let alpha_byte = bitmap[gy * metrics.width + gx];
                if alpha_byte == 0 { continue; }

                // fontdue bitmap row 0 = top of glyph bounding box.
                let px = cursor_x as i32 + gx as i32 + metrics.xmin;
                let py = baseline_y - metrics.ymin - metrics.height as i32 + 1 + gy as i32;

                if px >= 0 && px < text_width as i32 && py >= 0 && py < text_height as i32 {
                    let idx        = ((py as u32 * text_width + px as u32) * 4) as usize;
                    pixels[idx]     = r;
                    pixels[idx + 1] = g;
                    pixels[idx + 2] = b;
                    pixels[idx + 3] = ((alpha_byte as f32 / 255.0) * base_alpha * 255.0) as u8;
                }
            }
        }
        cursor_x += metrics.advance_width;
    }

    (pixels, text_width, text_height)
}

