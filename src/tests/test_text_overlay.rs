use crate::text_overlay::{TextLabelHandle, TextOverlay};
#[cfg(not(feature = "default-fonts"))]
use crate::text_overlay::TextLabel;
use crate::text_overlay::VerticalAlignment;

#[cfg(not(feature = "default-fonts"))]
#[test]
fn new_overlay_is_empty() {
    let overlay = TextOverlay::new();
    assert_eq!(overlay.label_count(), 0);
    assert_eq!(overlay.font_count(), 0);
}

#[cfg(feature = "default-fonts")]
#[test]
fn feature_default_fonts_new_overlay_is_not_empty() {
    let overlay = TextOverlay::new();
    assert_eq!(overlay.label_count(), 0);
    assert_ne!(overlay.font_count(), 0);
}

#[test]
fn load_invalid_font_returns_error() {
    let mut overlay = TextOverlay::new();
    assert!(overlay.add_font("x", b"not a font").is_err());
}

#[test]
fn add_font_rejects_empty_id() {
    let mut overlay = TextOverlay::new();
    assert!(overlay.add_font("", b"anything").is_err());
}

#[test]
fn add_font_rejects_duplicate_id() {
    let mut overlay = TextOverlay::new();
    let font_bytes = include_bytes!("./samples/fonts/Cause-Regular.ttf");

    overlay.add_font("dup", font_bytes).unwrap();

    let err = overlay.add_font("dup", b"x").unwrap_err();
    assert!(err.contains("is already registered"));
}

#[test]
fn has_font_returns_correct_values() {
    let mut overlay = TextOverlay::new();
    assert!(!overlay.has_font("roboto"));
    // add_font will fail on invalid bytes, so font_count stays 0
    let _ = overlay.add_font("roboto", b"fake");
    // Still false because parse failed
    assert!(!overlay.has_font("roboto"));
}
#[cfg(not(feature = "default-fonts"))]
#[test]
fn font_count_stays_zero_with_no_valid_fonts() {
    let overlay = TextOverlay::new();
    assert_eq!(overlay.font_count(), 0);
}

#[test]
fn builder_defaults() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("hello").build();
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.text, "hello");
    assert_eq!(label.x, 0.0);
    assert_eq!(label.y, 0.0);
    assert_eq!(label.font_size, 16.0);
    assert_eq!(label.color, [1.0, 1.0, 1.0, 1.0]);
    assert!(label.visible);
    assert_eq!(label.font_id, "");
}

#[test]
fn builder_at_sets_position() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("pos").at(50.0, 80.0).build();
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.x, 50.0);
    assert_eq!(label.y, 80.0);
}

#[test]
fn builder_with_color_sets_color() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("c").with_color([0.1, 0.2, 0.3, 0.4]).build();
    let c = h.label(&overlay).unwrap().color;
    assert!((c[0] - 0.1).abs() < 1e-5);
    assert!((c[3] - 0.4).abs() < 1e-5);
}

#[test]
fn builder_with_font_size_sets_size() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("fs").with_font_size(32.0).build();
    assert_eq!(h.label(&overlay).unwrap().font_size, 32.0);
}

#[test]
fn builder_with_font_sets_string_id() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("fi").with_font("alt").build();
    assert_eq!(h.label(&overlay).unwrap().font_id, "alt");
}

#[test]
fn builder_hidden_creates_invisible_label() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("hidden").hidden().build();
    assert!(!h.label(&overlay).unwrap().visible);
}

#[test]
fn builder_returns_unique_handles() {
    let mut overlay = TextOverlay::new();
    let a = overlay.add_label("A").build();
    let b = overlay.add_label("B").build();
    assert_ne!(a.id, b.id);
    assert_eq!(overlay.label_count(), 2);
}

#[test]
fn newly_built_label_is_dirty() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("d").build();
    assert!(h.label(&overlay).unwrap().dirty);
}

#[test]
fn handle_exists_returns_true_for_live_label() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("x").build();
    assert!(h.exists(&overlay));
}

#[test]
fn handle_label_returns_none_for_removed_label() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("gone").build();
    h.remove(&mut overlay);
    assert!(h.label(&overlay).is_none());
}

#[test]
fn handle_exists_returns_false_after_removal() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("x").build();
    h.remove(&mut overlay);
    assert!(!h.exists(&overlay));
}

#[test]
fn set_text_changes_text() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("old").build();
    assert!(h.set_text(&mut overlay, "new"));
    assert_eq!(h.label(&overlay).unwrap().text, "new");
}

#[test]
fn set_text_returns_false_for_missing_id() {
    let mut overlay = TextOverlay::new();
    let ghost = TextLabelHandle { id: 999 };
    assert!(!ghost.set_text(&mut overlay, "x"));
}

#[test]
fn move_to_changes_position() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("p").build();
    assert!(h.move_to(&mut overlay, 100.0, 200.0));
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.x, 100.0);
    assert_eq!(label.y, 200.0);
}

/// move_to must switch both alignments to Free.  Without this, the next
/// full re-bake would call resolve_screen_x / resolve_screen_y which use the
/// unchanged margin fields and silently snap the label back.
#[test]
fn move_to_switches_alignment_to_free() {
    use crate::text_label::{HorizontalAlignment, VerticalAlignment};
    let mut overlay = TextOverlay::new();
    // Start with non-Free alignments.
    let h = overlay
        .add_label("snap")
        .with_horizontal_alignment(HorizontalAlignment::Right)
        .with_vertical_alignment(VerticalAlignment::Bottom)
        .build();
    h.move_to(&mut overlay, 300.0, 150.0);
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.horizontal_alignment, HorizontalAlignment::Free,
        "move_to must switch horizontal_alignment to Free");
    assert_eq!(label.vertical_alignment, VerticalAlignment::Free,
        "move_to must switch vertical_alignment to Free");
}

/// move_to must update margin_x / margin_y to match the new absolute
/// position so that Free mode records the correct anchor.
#[test]
fn move_to_updates_margins() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("m").at(10.0, 20.0).build();
    h.move_to(&mut overlay, 300.0, 150.0);
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.margin_x, 300.0, "margin_x should be updated to the new x");
    assert_eq!(label.margin_y, 150.0, "margin_y should be updated to the new y");
}

/// After move_to the position_dirty flag must be set so the vertex buffer
/// is rebuilt without a full GPU texture re-upload.
#[test]
fn move_to_sets_position_dirty() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("pd").build();
    // Clear the dirty flag that build() sets.
    if let Some(l) = overlay.labels.get_mut(&h.id) { l.dirty = false; l.position_dirty = false; }
    h.move_to(&mut overlay, 50.0, 60.0);
    let label = h.label(&overlay).unwrap();
    assert!(label.position_dirty, "move_to must set position_dirty");
    assert!(!label.dirty,         "move_to must NOT trigger a full re-rasterise (dirty)");
}

/// A Center-aligned label moved with move_to must stay at the new position
/// (Free alignment) rather than being re-centered by resolve_screen_x.
#[test]
fn move_to_on_center_aligned_preserves_position() {
    use crate::text_label::HorizontalAlignment;
    let mut overlay = TextOverlay::new();
    let h = overlay
        .add_label("centre")
        .with_horizontal_alignment(HorizontalAlignment::Center)
        .build();
    h.move_to(&mut overlay, 42.0, 0.0);
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.x, 42.0);
    assert_eq!(label.horizontal_alignment, HorizontalAlignment::Free);
}

/// A Middle-aligned label moved with move_to must stay at the new vertical
/// position (Free) rather than being re-centred by resolve_screen_y.
#[test]
fn move_to_on_middle_aligned_preserves_vertical_position() {
    use crate::text_label::VerticalAlignment;
    let mut overlay = TextOverlay::new();
    let h = overlay
        .add_label("middle")
        .with_vertical_alignment(VerticalAlignment::Middle)
        .build();
    h.move_to(&mut overlay, 0.0, 99.0);
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.y, 99.0);
    assert_eq!(label.vertical_alignment, VerticalAlignment::Free);
}

/// move_to on an already-Free label preserves x/y and keeps Free alignment.
#[test]
fn move_to_on_free_label_stays_free() {
    use crate::text_label::{HorizontalAlignment, VerticalAlignment};
    let mut overlay = TextOverlay::new();
    let h = overlay
        .add_label("free")
        .with_horizontal_alignment(HorizontalAlignment::Free)
        .with_vertical_alignment(VerticalAlignment::Free)
        .build();
    h.move_to(&mut overlay, 7.0, 8.0);
    let label = h.label(&overlay).unwrap();
    assert_eq!(label.x, 7.0);
    assert_eq!(label.y, 8.0);
    assert_eq!(label.horizontal_alignment, HorizontalAlignment::Free);
    assert_eq!(label.vertical_alignment,   VerticalAlignment::Free);
}

/// move_to returns false for a label that has already been removed.
#[test]
fn move_to_returns_false_for_missing_label() {
    let mut overlay = TextOverlay::new();
    let ghost = TextLabelHandle { id: 9999 };
    assert!(!ghost.move_to(&mut overlay, 1.0, 2.0));
}

#[test]
fn set_color_changes_rgba() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("c").build();
    assert!(h.set_color(&mut overlay, [0.5, 0.6, 0.7, 0.8]));
    let c = h.label(&overlay).unwrap().color;
    assert!((c[0] - 0.5).abs() < 1e-5);
    assert!((c[3] - 0.8).abs() < 1e-5);
}

#[test]
fn set_font_size_changes_size() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("s").build();
    assert!(h.set_font_size(&mut overlay, 48.0));
    assert_eq!(h.label(&overlay).unwrap().font_size, 48.0);
}

#[test]
fn set_font_changes_index() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("f").build();
    assert!(h.set_font(&mut overlay, "mono"));
    assert_eq!(h.label(&overlay).unwrap().font_id, "mono");
}

#[test]
fn hide_and_show_toggle_visibility() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("v").build();
    assert!(h.label(&overlay).unwrap().visible);
    h.hide(&mut overlay);
    assert!(!h.label(&overlay).unwrap().visible);
    h.show(&mut overlay);
    assert!(h.label(&overlay).unwrap().visible);
}

#[test]
fn remove_returns_true_once_then_false() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("r").build();
    assert!(h.remove(&mut overlay));
    assert!(!h.remove(&mut overlay));
    assert_eq!(overlay.label_count(), 0);
}

#[test]
fn handle_is_copy() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("copy").build();
    let h2 = h;                            // copy, not move
    assert_eq!(h.id, h2.id);              // both still valid
    assert!(h.exists(&overlay));
}

#[test]
fn clear_removes_all_labels() {
    let mut overlay = TextOverlay::new();
    overlay.add_label("a").build();
    overlay.add_label("b").build();
    overlay.add_label("c").build();
    overlay.clear();
    assert_eq!(overlay.label_count(), 0);
}

#[test]
fn build_quad_produces_correct_geometry() {
    let (verts, indices) = TextOverlay::build_quad(5.0, 10.0, 40.0, 20.0);
    assert_eq!(verts.len(), 4);
    assert_eq!(indices.len(), 6);
    assert_eq!(verts[0].position, [5.0,  10.0, 0.0]);
    assert_eq!(verts[1].position, [45.0, 10.0, 0.0]);
    assert_eq!(verts[2].position, [45.0, 30.0, 0.0]);
    assert_eq!(verts[3].position, [5.0,  30.0, 0.0]);
    assert_eq!(verts[0].uv, [0.0, 0.0]);
    assert_eq!(verts[2].uv, [1.0, 1.0]);
}

#[test]
fn ortho_matrix_maps_origin_correctly() {
    use crate::pipeline::build_ortho_matrix;
    let m = build_ortho_matrix(800.0, 600.0);
    assert!((m[3][0] - (-1.0)).abs() < 1e-5);
    assert!((m[3][1] -   1.0 ).abs() < 1e-5);
}

#[test]
fn ortho_matrix_maps_bottom_right_correctly() {
    use crate::pipeline::build_ortho_matrix;
    let (w, h) = (800.0f32, 600.0f32);
    let m = build_ortho_matrix(w, h);
    let x_ndc = m[0][0] * w + m[3][0];
    let y_ndc = m[1][1] * h + m[3][1];
    assert!((x_ndc -  1.0).abs() < 1e-4, "x={x_ndc}");
    assert!((y_ndc - -1.0).abs() < 1e-4, "y={y_ndc}");
}

#[cfg(not(feature = "default-fonts"))]
#[test]
fn rasterize_returns_none_when_no_font_loaded() {
    let overlay = TextOverlay::new();
    let dummy = TextLabel {
        id: 0,
        text: "test".into(),
        x: 0.0,
        y: 0.0,
        font_size: 16.0,
        color: [1.0; 4],
        visible: true,
        font_id: String::new(),
        zindex: 0,
        horizontal_alignment: crate::text_label::HorizontalAlignment::Left,
        vertical_alignment: crate::text_label::VerticalAlignment::Top,
        dirty: true,
        position_dirty: false,
        rasterized_h: 0,
        rasterized_w: 0,
        rasterized_font_size: 0.0,
        rasterized_vp_w: 0.0,
        rasterized_vp_h: 0.0,
        margin_x: 0.0,
        margin_y: 0.0,
    };
    assert!(overlay.rasterize_label(&dummy).is_none());
}

#[test]
fn zindex_defaults_to_insertion_order() {
    let mut overlay = TextOverlay::new();
    let a = overlay.add_label("a").build();
    let b = overlay.add_label("b").build();
    let c = overlay.add_label("c").build();
    let za = a.label(&overlay).unwrap().zindex;
    let zb = b.label(&overlay).unwrap().zindex;
    let zc = c.label(&overlay).unwrap().zindex;
    assert!(za < zb, "first label should have lower zindex than second");
    assert!(zb < zc, "second label should have lower zindex than third");
}

#[test]
fn with_zindex_overrides_insertion_order() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("z").with_zindex(42).build();
    assert_eq!(h.label(&overlay).unwrap().zindex, 42);
}

#[test]
fn set_zindex_updates_value() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("z").build();
    assert!(h.set_zindex(&mut overlay, -5));
    assert_eq!(h.label(&overlay).unwrap().zindex, -5);
}

#[test]
fn set_zindex_returns_false_for_missing_label() {
    let mut overlay = TextOverlay::new();
    let ghost = TextLabelHandle { id: 999 };
    assert!(!ghost.set_zindex(&mut overlay, 10));
}

#[test]
fn builder_default_vertical_alignment_is_top() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("v").build();
    assert_eq!(
        h.label(&overlay).unwrap().vertical_alignment,
        VerticalAlignment::Top
    );
}

#[test]
fn builder_with_vertical_alignment_middle() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("m")
        .with_vertical_alignment(VerticalAlignment::Middle)
        .build();
    assert_eq!(
        h.label(&overlay).unwrap().vertical_alignment,
        VerticalAlignment::Middle
    );
}

#[test]
fn builder_with_vertical_alignment_bottom() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("b")
        .with_vertical_alignment(VerticalAlignment::Bottom)
        .build();
    assert_eq!(
        h.label(&overlay).unwrap().vertical_alignment,
        VerticalAlignment::Bottom
    );
}

#[test]
fn set_vertical_alignment_updates_value() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("x").build();
    assert!(h.set_vertical_alignment(&mut overlay, VerticalAlignment::Bottom));
    assert_eq!(
        h.label(&overlay).unwrap().vertical_alignment,
        VerticalAlignment::Bottom
    );
}

#[test]
fn set_vertical_alignment_returns_false_for_missing_label() {
    let mut overlay = TextOverlay::new();
    let ghost = TextLabelHandle { id: 999 };
    assert!(!ghost.set_vertical_alignment(&mut overlay, VerticalAlignment::Middle));
}

#[test]
fn vertical_alignment_to_u8_roundtrip() {
    use crate::text_label::VerticalAlignment;
    assert_eq!(VerticalAlignment::from_u8(VerticalAlignment::Top.to_u8()),    VerticalAlignment::Top);
    assert_eq!(VerticalAlignment::from_u8(VerticalAlignment::Middle.to_u8()), VerticalAlignment::Middle);
    assert_eq!(VerticalAlignment::from_u8(VerticalAlignment::Bottom.to_u8()), VerticalAlignment::Bottom);
    // Unknown values fall back to Top.
    assert_eq!(VerticalAlignment::from_u8(99), VerticalAlignment::Top);
}

