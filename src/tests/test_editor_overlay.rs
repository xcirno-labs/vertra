use crate::editor::{EditorEvent, EditorState, EditorStateEvent, LabelDragKind, LabelDragState, LabelInspectorData};
use crate::text_overlay::TextOverlay;

#[test]
fn label_drag_moves_label_via_motion_delta() {
    let mut overlay = TextOverlay::new();
    let handle = overlay
        .add_label("drag me")
        .at(10.0, 20.0)
        .with_font_size(12.0)
        .build();
    let label = handle.label(&overlay).unwrap().clone();
    let mut editor = EditorState::new(800.0, 600.0);

    editor.input.cursor_x = 10.0;
    editor.input.cursor_y = 20.0;
    editor.selected_label = Some(LabelInspectorData::from_label(&label));
    editor.label_drag = Some(LabelDragState {
        label_id: label.id,
        kind: LabelDragKind::Move,
        start_cursor: [10.0, 20.0],
        start_pos: [label.x, label.y],
        start_size: label.font_size,
    });

    // Motion during a drag: label moves but no event is returned.
    let ev = editor.process_overlay(&mut overlay, &EditorEvent::MouseMotionDelta { dx: 3.0, dy: -2.0 });
    let moved = handle.label(&overlay).unwrap();

    assert_eq!((moved.x, moved.y), (13.0, 18.0));
    assert!(moved.dirty);
    assert!(ev.is_none(), "motion during drag should not fire an event");
}

#[test]
fn label_resize_uses_motion_delta() {
    let mut overlay = TextOverlay::new();
    let handle = overlay
        .add_label("resize me")
        .at(10.0, 20.0)
        .with_font_size(12.0)
        .build();
    let label = handle.label(&overlay).unwrap().clone();
    let mut editor = EditorState::new(800.0, 600.0);

    editor.input.cursor_x = 40.0;
    editor.input.cursor_y = 20.0;
    editor.selected_label = Some(LabelInspectorData::from_label(&label));
    editor.label_drag = Some(LabelDragState {
        label_id: label.id,
        kind: LabelDragKind::Resize,
        start_cursor: [40.0, 20.0],
        start_pos: [label.x, label.y],
        start_size: label.font_size,
    });

    // dx=4 -> new_size = 12.0 + 4*0.5 = 14.0
    let ev = editor.process_overlay(&mut overlay, &EditorEvent::MouseMotionDelta { dx: 4.0, dy: 5.0 });
    let resized = handle.label(&overlay).unwrap();

    assert!((resized.font_size - 14.0).abs() < 1e-6);
    assert!(resized.dirty);
    assert!(ev.is_none(), "motion during resize should not fire an event");
}

#[test]
fn label_drag_end_fires_on_mouse_up() {
    let mut overlay = TextOverlay::new();
    let handle = overlay
        .add_label("end me")
        .at(10.0, 20.0)
        .with_font_size(12.0)
        .build();
    let label = handle.label(&overlay).unwrap().clone();
    let mut editor = EditorState::new(800.0, 600.0);

    editor.selected_label = Some(LabelInspectorData::from_label(&label));
    editor.label_drag = Some(LabelDragState {
        label_id: label.id,
        kind: LabelDragKind::Move,
        start_cursor: [10.0, 20.0],
        start_pos: [label.x, label.y],
        start_size: label.font_size,
    });

    let ev = editor.process_overlay(
        &mut overlay,
        &EditorEvent::MouseButton { left: Some(false), middle: None, right: None },
    );

    assert!(editor.label_drag.is_none(), "drag should be cleared on mouse up");
    assert!(
        matches!(ev, Some(EditorStateEvent::LabelDragEnd(_))),
        "LabelDragEnd should be fired on mouse up"
    );
}

#[test]
fn raw_mouse_motion_does_not_fire_event_without_drag() {
    let mut overlay = TextOverlay::new();
    let handle = overlay.add_label("raw motion").at(10.0, 20.0).build();
    let label = handle.label(&overlay).unwrap().clone();
    let mut editor = EditorState::new(800.0, 600.0);

    editor.selected_label = Some(LabelInspectorData::from_label(&label));
    // No label_drag set, simulates motion without a drag in progress.

    let ev = editor.process_overlay(&mut overlay, &EditorEvent::MouseMotionDelta { dx: 5.0, dy: -3.0 });
    let unchanged = handle.label(&overlay).unwrap();

    assert_eq!((unchanged.x, unchanged.y), (10.0, 20.0));
    assert!(ev.is_none());
}
