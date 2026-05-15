/// VTR v3 text-overlay serialization tests.
use std::io::Cursor;
use crate::camera::Camera;
use crate::objects::Object;
use crate::text_overlay::{TextOverlay, HorizontalAlignment, VerticalAlignment};
use crate::vtr;
use crate::world::World;

fn test_camera() -> Camera { Camera::new() }

fn custom_camera() -> Camera {
    Camera {
        eye: [1.0, 2.0, 3.0], target: [4.0, 5.0, 6.0], up: [0.0, 1.0, 0.0],
        aspect: 16.0 / 9.0, fov: 60.0, znear: 0.01, zfar: 500.0,
        lr_rot: 45.0, ud_rot: -15.0,
    }
}

fn assert_cameras_eq(a: &Camera, b: &Camera) {
    assert_eq!(a.eye,    b.eye,    "eye");
    assert_eq!(a.target, b.target, "target");
    assert_eq!(a.fov,    b.fov,    "fov");
}

fn roundtrip_with_overlay(
    camera: &Camera,
    world: &World,
    overlay: &TextOverlay,
) -> vtr::SceneData {
    let mut buf = Vec::new();
    vtr::write_scene(&mut buf, camera, world, overlay).expect("write_scene failed");
    let mut cur = Cursor::new(&buf);
    vtr::read(&mut cur).expect("read failed")
}

#[test]
fn v3_format_version_field() {
    let mut buf = Vec::new();
    vtr::write(&mut buf, &test_camera(), &World::new()).unwrap();
    let fv = u16::from_le_bytes([buf[4], buf[5]]);
    assert_eq!(fv, vtr::FORMAT_VERSION, "must emit FORMAT_VERSION = 3");
}

#[test]
fn v3_empty_overlay_roundtrip() {
    let overlay = TextOverlay::new();
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.label_count(), 0);
}

#[test]
fn v3_single_label_roundtrip() {
    let mut overlay = TextOverlay::new();
    let handle = overlay
        .add_label("Hello VTR")
        .at(100.0, 200.0)
        .with_font_size(24.0)
        .with_color([1.0, 0.5, 0.0, 1.0])
        .build();

    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.label_count(), 1);
    let lbl = &data.text_overlay.labels[&handle.id];
    assert_eq!(lbl.text, "Hello VTR");
    assert_eq!(lbl.x, 100.0);
    assert_eq!(lbl.y, 200.0);
    assert_eq!(lbl.font_size, 24.0);
    assert_eq!(lbl.color, [1.0, 0.5, 0.0, 1.0]);
    assert!(lbl.visible);
}

#[test]
fn v3_label_alignment_roundtrip() {
    let mut overlay = TextOverlay::new();
    let l = overlay.add_label("L").with_alignment(HorizontalAlignment::Left).build();
    let c = overlay.add_label("C").with_alignment(HorizontalAlignment::Center).build();
    let r = overlay.add_label("R").with_alignment(HorizontalAlignment::Right).build();

    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.labels[&l.id].alignment, HorizontalAlignment::Left);
    assert_eq!(data.text_overlay.labels[&c.id].alignment, HorizontalAlignment::Center);
    assert_eq!(data.text_overlay.labels[&r.id].alignment, HorizontalAlignment::Right);
}

#[test]
fn v3_vertical_alignment_roundtrip() {
    let mut overlay = TextOverlay::new();
    let t = overlay.add_label("T").with_vertical_alignment(VerticalAlignment::Top).build();
    let m = overlay.add_label("M").with_vertical_alignment(VerticalAlignment::Middle).build();
    let b = overlay.add_label("B").with_vertical_alignment(VerticalAlignment::Bottom).build();

    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.labels[&t.id].vertical_alignment, VerticalAlignment::Top);
    assert_eq!(data.text_overlay.labels[&m.id].vertical_alignment, VerticalAlignment::Middle);
    assert_eq!(data.text_overlay.labels[&b.id].vertical_alignment, VerticalAlignment::Bottom);
}

#[test]
fn v3_combined_alignment_roundtrip() {
    // Verify both h- and v-alignment survive a full round-trip together.
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("BR")
        .with_alignment(HorizontalAlignment::Right)
        .with_vertical_alignment(VerticalAlignment::Bottom)
        .at(20.0, 20.0)
        .with_font_size(18.0)
        .build();

    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    let lbl = &data.text_overlay.labels[&h.id];
    assert_eq!(lbl.alignment, HorizontalAlignment::Right);
    assert_eq!(lbl.vertical_alignment, VerticalAlignment::Bottom);
    assert_eq!(lbl.x, 20.0);
    assert_eq!(lbl.y, 20.0);
}

#[test]
fn v3_label_hidden_roundtrip() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("Hidden").hidden().build();
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert!(!data.text_overlay.labels[&h.id].visible);
}

#[test]
fn v3_label_zindex_roundtrip() {
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label("Z").with_zindex(42).build();
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.labels[&h.id].zindex, 42);
}

#[test]
fn v3_label_unicode_text_roundtrip() {
    let text = "太陽 ☀ Soleil 🌍".to_string();
    let mut overlay = TextOverlay::new();
    let h = overlay.add_label(&text).build();
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.labels[&h.id].text, text);
}

#[test]
fn v3_multiple_labels_next_id_preserved() {
    let mut overlay = TextOverlay::new();
    overlay.add_label("A").build();
    overlay.add_label("B").build();
    overlay.add_label("C").build();
    let expected = overlay.next_id;
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    assert_eq!(data.text_overlay.next_id, expected);
    assert_eq!(data.text_overlay.label_count(), 3);
}

#[test]
fn v3_labels_loaded_as_dirty() {
    let mut overlay = TextOverlay::new();
    overlay.add_label("Dirty check").build();
    let data = roundtrip_with_overlay(&test_camera(), &World::new(), &overlay);
    for lbl in data.text_overlay.labels.values() {
        assert!(lbl.dirty, "loaded label must be dirty for re-rasterisation");
    }
}

#[test]
fn v3_label_and_world_together_roundtrip() {
    let mut world = World::new();
    world.spawn_object(Object::default(), None);
    let mut overlay = TextOverlay::new();
    overlay.add_label("Score: 0").at(10.0, 10.0).with_font_size(32.0).build();

    let camera = custom_camera();
    let data = roundtrip_with_overlay(&camera, &world, &overlay);
    assert_eq!(data.world.objects.len(), 1);
    assert_eq!(data.text_overlay.label_count(), 1);
    assert_cameras_eq(&camera, &data.camera);
}

#[test]
fn v3_idempotent_with_labels() {
    let mut world = World::new();
    world.spawn_object(Object::default(), None);
    let mut overlay = TextOverlay::new();
    overlay
        .add_label("Hi")
        .at(5.0, 5.0)
        .with_font_size(16.0)
        .with_alignment(HorizontalAlignment::Center)
        .build();

    let camera = custom_camera();
    let mut b1 = Vec::new();
    vtr::write_scene(&mut b1, &camera, &world, &overlay).unwrap();
    let d = vtr::read(&mut Cursor::new(&b1)).unwrap();
    let mut b2 = Vec::new();
    vtr::write_scene(&mut b2, &d.camera, &d.world, &d.text_overlay).unwrap();
    assert_eq!(b1, b2, "write_scene output must be idempotent");
}

#[test]
fn v2_file_reads_with_empty_overlay() {
    // Build a minimal valid V2 binary by hand (empty scene, no text section).
    let cam = Camera::new();
    let mut buf = Vec::new();
    let wf = |b: &mut Vec<u8>, v: f32| b.extend_from_slice(&v.to_le_bytes());
    let wu = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    let wu16 = |b: &mut Vec<u8>, v: u16| b.extend_from_slice(&v.to_le_bytes());

    buf.extend_from_slice(&vtr::MAGIC);
    wu16(&mut buf, 2);  // format_version = 2
    wu16(&mut buf, vtr::ENGINE_VERSION_MAJOR);
    wu16(&mut buf, vtr::ENGINE_VERSION_MINOR);
    wu16(&mut buf, vtr::ENGINE_VERSION_PATCH);
    wu(&mut buf, 0);    // flags
    wu(&mut buf, 0);    // object_count = 0
    // Camera block (9 f32s = 36 + 24 = 60 bytes)
    for v in [cam.eye[0], cam.eye[1], cam.eye[2],
              cam.target[0], cam.target[1], cam.target[2],
              cam.up[0], cam.up[1], cam.up[2],
              cam.aspect, cam.fov, cam.znear, cam.zfar, cam.lr_rot, cam.ud_rot] {
        wf(&mut buf, v);
    }
    wu(&mut buf, 0); // roots_count = 0
    // No text overlay section (V2 format stops here).

    let data = vtr::read(&mut Cursor::new(&buf)).expect("V2 must be readable");
    assert_eq!(data.text_overlay.label_count(), 0, "V2 -> empty text overlay");
}

#[test]
fn v3_file_reads_with_top_vertical_alignment() {
    // Build a minimal V3 binary with one label.  V3 includes the vertical_alignment
    // byte; write 0 (Top) explicitly and confirm it round-trips correctly.
    let cam = Camera::new();
    let mut buf = Vec::new();
    let wf   = |b: &mut Vec<u8>, v: f32| b.extend_from_slice(&v.to_le_bytes());
    let wu   = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
    let wi32 = |b: &mut Vec<u8>, v: i32| b.extend_from_slice(&v.to_le_bytes());
    let wu16 = |b: &mut Vec<u8>, v: u16| b.extend_from_slice(&v.to_le_bytes());

    buf.extend_from_slice(&vtr::MAGIC);
    wu16(&mut buf, 3);  // format_version = 3
    wu16(&mut buf, vtr::ENGINE_VERSION_MAJOR);
    wu16(&mut buf, vtr::ENGINE_VERSION_MINOR);
    wu16(&mut buf, vtr::ENGINE_VERSION_PATCH);
    wu(&mut buf, 0);   // flags
    wu(&mut buf, 0);   // object_count
    for v in [cam.eye[0], cam.eye[1], cam.eye[2],
              cam.target[0], cam.target[1], cam.target[2],
              cam.up[0], cam.up[1], cam.up[2],
              cam.aspect, cam.fov, cam.znear, cam.zfar, cam.lr_rot, cam.ud_rot] {
        wf(&mut buf, v);
    }
    wu(&mut buf, 0);  // roots_count
    // V3 text overlay section: next_id=1, label_count=1
    wu(&mut buf, 1);  // overlay_next_id
    wu(&mut buf, 1);  // label_count
    // One label: id=0, x=10, y=20, font_size=16, color=[1,1,1,1],
    //            visible=1, zindex=0, alignment=2 (Right), vertical_alignment=0 (Top)
    wu(&mut buf, 0);   // id
    wf(&mut buf, 10.0); // x (= margin_x)
    wf(&mut buf, 20.0); // y (= margin_y)
    wf(&mut buf, 16.0); // font_size
    for _ in 0..4 { wf(&mut buf, 1.0); }  // color
    buf.push(1u8);     // visible
    wi32(&mut buf, 0); // zindex
    buf.push(2u8);     // alignment = Right
    buf.push(0u8);     // vertical_alignment = Top (V3 includes this byte)
    // font_id: empty string
    wu16(&mut buf, 0);
    // text: "Hi" (2 bytes)
    wu(&mut buf, 2);
    buf.extend_from_slice(b"Hi");

    let data = vtr::read(&mut Cursor::new(&buf)).expect("V3 must be readable");
    assert_eq!(data.text_overlay.label_count(), 1);
    let lbl = data.text_overlay.labels.values().next().unwrap();
    assert_eq!(lbl.text, "Hi");
    assert_eq!(lbl.alignment, HorizontalAlignment::Right,
               "alignment should be Right as written");
    assert_eq!(lbl.vertical_alignment, VerticalAlignment::Top,
        "vertical_alignment byte 0 must decode to Top");
}


