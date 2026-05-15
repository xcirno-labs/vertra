//! # VTR Binary Scene Format
//!
//! A compact, lossless, little-endian binary format for Vertra scene files.
//!
//! ## File Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │  HEADER  (20 bytes)                                          │
//! │  [0..4]   magic:          b"VTR\x00"                         │
//! │  [4..6]   format_version: u16 LE  (= FORMAT_VERSION)         │
//! │  [6..8]   engine_major:   u16 LE                             │
//! │  [8..10]  engine_minor:   u16 LE                             │
//! │  [10..12] engine_patch:   u16 LE                             │
//! │  [12..16] flags:          u32 LE  (= 0, reserved)            │
//! │  [16..20] object_count:   u32 LE                             │
//! ├──────────────────────────────────────────────────────────────┤
//! │  CAMERA BLOCK  (60 bytes)                                    │
//! │  eye[3], target[3], up[3]: f32 LE  (36 bytes)                │
//! │  aspect, fov, znear, zfar, lr_rot, ud_rot: f32 LE (24 bytes) │
//! ├──────────────────────────────────────────────────────────────┤
//! │  ROOTS SECTION                                               │
//! │  roots_count: u32 LE                                         │
//! │  root_ids:    u32 LE * roots_count                           │
//! ├──────────────────────────────────────────────────────────────┤
//! │  OBJECTS SECTION  (object_count entries, ascending id order) │
//! │  Per object:                                                 │
//! │    id:             u32 LE                                    │
//! │    parent_id:      u32 LE  (u32::MAX = no parent)            │
//! │    name_len:       u16 LE                                    │
//! │    str_id_len:     u16 LE                                    │
//! │    str_id:         utf-8 bytes [str_id_len]                  │
//! │    name:           utf-8 bytes [name_len]                    │
//! │    position[3]:    f32 LE * 3                                │
//! │    rotation[3]:    f32 LE * 3                                │
//! │    scale[3]:       f32 LE * 3                                │
//! │    color[4]:       f32 LE * 4                                │
//! │    geometry_tag:   u8                                        │
//! │      0=None  1=Cube  2=Box  3=Plane                          │
//! │      4=Pyramid  5=Capsule  6=Sphere                          │
//! │    geometry_data:  (varies by tag)                           │
//! │    texture_path_len: u16 LE  (0 = no texture)                │
//! │    texture_path:  utf-8 bytes [texture_path_len]             │
//! │    children_count: u32 LE                                    │
//! │    children:       u32 LE * children_count                   │
//! ├──────────────────────────────────────────────────────────────┤
//! │  TEXT OVERLAY SECTION  (V3+, absent in V2)                   │
//! │  overlay_next_id: u32 LE                                     │
//! │  label_count:     u32 LE                                     │
//! │  Per label:                                                  │
//! │    id:          u32 LE                                       │
//! │    x:           f32 LE                                       │
//! │    y:           f32 LE                                       │
//! │    font_size:   f32 LE                                       │
//! │    color[4]:    f32 LE × 4                                   │
//! │    visible:     u8  (1 = visible)                            │
//! │    zindex:      i32 LE                                       │
//! │    alignment:   u8  (0=Left 1=Center 2=Right)                │
//! │    font_id_len: u16 LE                                       │
//! │    font_id:     utf-8 bytes [font_id_len]                    │
//! │    text_len:    u32 LE                                       │
//! │    text:        utf-8 bytes [text_len]                       │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Minimum valid V3 file (header + empty camera + no objects + empty overlay):
//! **92 bytes**.  V2 files (84 bytes, no text section) remain readable and
//! produce an empty text overlay on load.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::objects::Object;
use crate::text_label::{HorizontalAlignment, VerticalAlignment, TextLabel};
use crate::text_overlay::TextOverlay;
use crate::transform::Transform;
use crate::world::World;

// Constants
/// Magic bytes that identify every valid VTR file.
pub const MAGIC: [u8; 4] = [0x56, 0x54, 0x52, 0x00]; // "VTR\0"

/// Current binary layout version.
/// V2 files (no text overlay section) remain readable as a backward-compat path.
pub const FORMAT_VERSION: u16 = 3;

/// Engine version embedded in the header for informational purposes.
pub const ENGINE_VERSION_MAJOR: u16 = 0;
pub const ENGINE_VERSION_MINOR: u16 = 2;
pub const ENGINE_VERSION_PATCH: u16 = 0;

/// Sentinel stored in `parent_id` when an object has no parent.
const NO_PARENT: u32 = u32::MAX;

// Public types

/// All scene data loaded from (or prepared for) a `.vtr` file.
#[derive(Debug)]
pub struct SceneData {
    pub camera: Camera,
    pub world: World,
    pub text_overlay: TextOverlay,
}

/// Metadata from the file header — readable without parsing the full scene.
#[derive(Debug, Clone, PartialEq)]
pub struct VtrHeader {
    /// Version of the binary layout.
    pub format_version: u16,
    /// Engine major version that wrote this file.
    pub engine_major: u16,
    /// Engine minor version that wrote this file.
    pub engine_minor: u16,
    /// Engine patch version that wrote this file.
    pub engine_patch: u16,
    /// Number of objects in the scene.
    pub object_count: u32,
}

impl VtrHeader {
    /// Human-readable engine version string, e.g. `"0.1.0"`.
    pub fn engine_version_string(&self) -> String {
        format!("{}.{}.{}", self.engine_major, self.engine_minor, self.engine_patch)
    }
}

/// Errors that can occur when reading or writing `.vtr` files.
#[derive(Debug)]
pub enum VtrError {
    Io(io::Error),
    /// The first four bytes do not match `b"VTR\0"`.
    InvalidMagic,
    /// The `format_version` field is not recognised by this implementation.
    UnsupportedVersion { found: u16 },
    /// An object's name field contained invalid UTF-8.
    InvalidUtf8(std::string::FromUtf8Error),
    /// An unknown `geometry_tag` byte was encountered.
    UnknownGeometryTag(u8),
    /// An object's `texture_path` is longer than `u16::MAX` bytes.
    TexturePathTooLong { len: usize },
    /// A label's `text` field exceeds `u32::MAX` bytes.
    LabelTextTooLong { len: usize },
}

impl std::fmt::Display for VtrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VtrError::Io(e) => write!(f, "I/O error: {e}"),
            VtrError::InvalidMagic => {
                write!(f, "Not a valid VTR file (magic bytes mismatch)")
            }
            VtrError::UnsupportedVersion { found } => {
                write!(
                    f,
                    "Unsupported VTR format version {found} \
                     (this build supports versions 2-{FORMAT_VERSION})"
                )
            }
            VtrError::InvalidUtf8(e) => write!(f, "Invalid UTF-8 in object name: {e}"),
            VtrError::UnknownGeometryTag(tag) => {
                write!(f, "Unknown geometry tag byte: {tag:#04x}")
            }
            VtrError::TexturePathTooLong { len } => {
                write!(
                    f,
                    "texture_path is {len} bytes, which exceeds the maximum \
                     of {} bytes allowed by the VTR u16 length field",
                    u16::MAX
                )
            }
            VtrError::LabelTextTooLong { len } => {
                write!(
                    f,
                    "label text is {len} bytes, which exceeds the maximum \
                     of {} bytes allowed by the VTR u32 length field",
                    u32::MAX
                )
            }
        }
    }
}

impl std::error::Error for VtrError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VtrError::Io(e) => Some(e),
            VtrError::InvalidUtf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for VtrError {
    fn from(e: io::Error) -> Self {
        VtrError::Io(e)
    }
}

impl From<std::string::FromUtf8Error> for VtrError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        VtrError::InvalidUtf8(e)
    }
}

// Byte helpers

#[inline]
fn w_u16(w: &mut impl Write, v: u16) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

#[inline]
fn w_u32(w: &mut impl Write, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

#[inline]
fn w_f32(w: &mut impl Write, v: f32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

#[inline]
fn w_i32(w: &mut impl Write, v: i32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

#[inline]
fn w_f32x3(w: &mut impl Write, v: [f32; 3]) -> io::Result<()> {
    w_f32(w, v[0])?;
    w_f32(w, v[1])?;
    w_f32(w, v[2])
}
#[inline]
fn w_f32x4(w: &mut impl Write, v: [f32; 4]) -> io::Result<()> {
    w_f32(w, v[0])?;
    w_f32(w, v[1])?;
    w_f32(w, v[2])?;
    w_f32(w, v[3])
}

#[inline] fn r_u8 (r: &mut impl Read) -> io::Result<u8>  {
    let mut b=[0u8;1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}
#[inline] fn r_u16(r: &mut impl Read) -> io::Result<u16> {
    let mut b=[0u8;2];
    r.read_exact(&mut b)?;
    Ok(u16::from_le_bytes(b)) }
#[inline] fn r_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b=[0u8;4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b)) }
#[inline] fn r_i32(r: &mut impl Read) -> io::Result<i32> {
    let mut b=[0u8;4];
    r.read_exact(&mut b)?;
    Ok(i32::from_le_bytes(b))
}
#[inline] fn r_f32(r: &mut impl Read) -> io::Result<f32> {
    let mut b=[0u8;4];
    r.read_exact(&mut b)?;
    Ok(f32::from_le_bytes(b))
}

#[inline]
fn r_f32x3(r: &mut impl Read) -> io::Result<[f32; 3]> {
    Ok([r_f32(r)?, r_f32(r)?, r_f32(r)?])
}
#[inline]
fn r_f32x4(r: &mut impl Read) -> io::Result<[f32; 4]> {
    Ok([r_f32(r)?, r_f32(r)?, r_f32(r)?, r_f32(r)?])
}

// Geometry encoding

/// Geometry tag byte values.
mod tag {
    pub const NONE:    u8 = 0;
    pub const CUBE:    u8 = 1;
    pub const BOX:     u8 = 2;
    pub const PLANE:   u8 = 3;
    pub const PYRAMID: u8 = 4;
    pub const CAPSULE: u8 = 5;
    pub const SPHERE:  u8 = 6;
}

fn write_geometry(w: &mut impl Write, geom: &Option<Geometry>) -> io::Result<()> {
    match geom {
        None => w.write_all(&[tag::NONE]),
        Some(Geometry::Cube { size }) => {
            w.write_all(&[tag::CUBE])?;
            w_f32(w, *size)
        }
        Some(Geometry::Box { width, height, depth }) => {
            w.write_all(&[tag::BOX])?;
            w_f32(w, *width)?;
            w_f32(w, *height)?;
            w_f32(w, *depth)
        }
        Some(Geometry::Plane { size }) => {
            w.write_all(&[tag::PLANE])?;
            w_f32(w, *size)
        }
        Some(Geometry::Pyramid { base_size, height }) => {
            w.write_all(&[tag::PYRAMID])?;
            w_f32(w, *base_size)?;
            w_f32(w, *height)
        }
        Some(Geometry::Capsule { radius, height, subdivisions }) => {
            w.write_all(&[tag::CAPSULE])?;
            w_f32(w, *radius)?;
            w_f32(w, *height)?;
            w_u32(w, *subdivisions as u32)
        }
        Some(Geometry::Sphere { radius, subdivisions }) => {
            w.write_all(&[tag::SPHERE])?;
            w_f32(w, *radius)?;
            w_u32(w, *subdivisions as u32)
        }
    }
}

fn read_geometry(r: &mut impl Read) -> Result<Option<Geometry>, VtrError> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    match buf[0] {
        tag::NONE => Ok(None),
        tag::CUBE => Ok(Some(Geometry::Cube { size: r_f32(r)? })),
        tag::BOX => Ok(Some(Geometry::Box {
            width: r_f32(r)?,
            height: r_f32(r)?,
            depth: r_f32(r)?,
        })),
        tag::PLANE => Ok(Some(Geometry::Plane { size: r_f32(r)? })),
        tag::PYRAMID => Ok(Some(Geometry::Pyramid {
            base_size: r_f32(r)?,
            height: r_f32(r)?,
        })),
        tag::CAPSULE => Ok(Some(Geometry::Capsule {
            radius: r_f32(r)?,
            height: r_f32(r)?,
            subdivisions: r_u32(r)? as usize,
        })),
        tag::SPHERE => Ok(Some(Geometry::Sphere {
            radius: r_f32(r)?,
            subdivisions: r_u32(r)? as usize,
        })),
        unknown => Err(VtrError::UnknownGeometryTag(unknown)),
    }
}

fn write_text_overlay(w: &mut impl Write, overlay: &TextOverlay) -> Result<(), VtrError> {
    w_u32(w, overlay.next_id as u32)?;
    w_u32(w, overlay.labels.len() as u32)?;

    // Sort by id for deterministic output.
    let mut ids: Vec<usize> = overlay.labels.keys().copied().collect();
    ids.sort_unstable();

    for id in ids {
        let lbl = &overlay.labels[&id];
        w_u32(w, lbl.id as u32)?;
        // V3+: write semantic margins so that round-trips are correct even after
        // a label has been baked (label.x/y hold absolute coords after bake).
        w_f32(w, lbl.margin_x)?;
        w_f32(w, lbl.margin_y)?;
        w_f32(w, lbl.font_size)?;
        w_f32x4(w, lbl.color)?;
        w.write_all(&[lbl.visible as u8])?;
        w_i32(w, lbl.zindex)?;
        w.write_all(&[lbl.alignment.to_u8()])?;
        w.write_all(&[lbl.vertical_alignment.to_u8()])?;

        let font_id_bytes = lbl.font_id.as_bytes();
        let font_id_len = font_id_bytes.len().min(u16::MAX as usize) as u16;
        w_u16(w, font_id_len)?;
        w.write_all(&font_id_bytes[..font_id_len as usize])?;

        let text_bytes = lbl.text.as_bytes();
        if text_bytes.len() > u32::MAX as usize {
            return Err(VtrError::LabelTextTooLong { len: text_bytes.len() });
        }
        w_u32(w, text_bytes.len() as u32)?;
        w.write_all(text_bytes)?;
    }
    Ok(())
}

fn read_text_overlay(r: &mut impl Read, format_version: u16) -> Result<TextOverlay, VtrError> {
    let next_id      = r_u32(r)? as usize;
    let label_count  = r_u32(r)? as usize;
    let mut labels   = HashMap::with_capacity(label_count);

    for _ in 0..label_count {
        let id        = r_u32(r)? as usize;
        let x         = r_f32(r)?;
        let y         = r_f32(r)?;
        let font_size = r_f32(r)?;
        let color     = r_f32x4(r)?;
        let visible   = r_u8(r)? != 0;
        let zindex    = r_i32(r)?;
        let alignment = HorizontalAlignment::from_u8(r_u8(r)?);
        let vertical_alignment = if format_version >= 3 {
            VerticalAlignment::from_u8(r_u8(r)?)
        } else {
            VerticalAlignment::Top
        };

        let font_id_len = r_u16(r)? as usize;
        let mut font_id_bytes = vec![0u8; font_id_len];
        r.read_exact(&mut font_id_bytes)?;
        let font_id = String::from_utf8(font_id_bytes)?;

        let text_len = r_u32(r)? as usize;
        let mut text_bytes = vec![0u8; text_len];
        r.read_exact(&mut text_bytes)?;
        let text = String::from_utf8(text_bytes)?;

        labels.insert(id, TextLabel {
            id, text, font_size, color, visible, font_id,
            zindex, alignment, vertical_alignment,
            // V3+: x/y fields in file are semantic margins (margin_x/margin_y).
            x,
            y,
            margin_x:             x,
            margin_y:             y,
            dirty:                true,
            position_dirty:       false,
            rasterized_w:         0,
            rasterized_h:         0,
            rasterized_font_size: 0.0,
            rasterized_vp_w:      0.0,
            rasterized_vp_h:      0.0,
        });
    }

    Ok(TextOverlay { labels, next_id, fonts: Vec::new() })
}

/// Read only the 20-byte file header from any [`Read`] source.
///
/// Useful for quickly inspecting engine/format version info without loading
/// the entire scene.  Accepts V2 and V3 files.
pub fn read_header(r: &mut impl Read) -> Result<VtrHeader, VtrError> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(VtrError::InvalidMagic);
    }
    let format_version = r_u16(r)?;
    // Accept V2 (no text overlay) and V3 (with text overlay) up to FORMAT_VERSION.
    if format_version < 2 || format_version > FORMAT_VERSION {
        return Err(VtrError::UnsupportedVersion { found: format_version });
    }
    let engine_major = r_u16(r)?;
    let engine_minor = r_u16(r)?;
    let engine_patch = r_u16(r)?;
    let _flags       = r_u32(r)?; // reserved
    let object_count = r_u32(r)?;

    Ok(VtrHeader { format_version, engine_major, engine_minor, engine_patch, object_count })
}

/// Serialize a complete scene (with empty text overlay) to any [`Write`] sink.
///
/// Writes a V3 file.  Use [`write_scene`] to include text labels.
pub fn write(w: &mut impl Write, camera: &Camera, world: &World) -> Result<(), VtrError> {
    write_scene(w, camera, world, &TextOverlay { labels: HashMap::new(), next_id: 0, fonts: Vec::new() })
}

/// Serialize a complete scene **with** text overlay to any [`Write`] sink.
///
/// Objects are written in ascending `id` order so the binary output is
/// deterministic for the same scene regardless of HashMap iteration order.
pub fn write_scene(w: &mut impl Write, camera: &Camera, world: &World, overlay: &TextOverlay) -> Result<(), VtrError> {
    w.write_all(&MAGIC)?;
    w_u16(w, FORMAT_VERSION)?;
    w_u16(w, ENGINE_VERSION_MAJOR)?;
    w_u16(w, ENGINE_VERSION_MINOR)?;
    w_u16(w, ENGINE_VERSION_PATCH)?;
    w_u32(w, 0)?; // flags - reserved
    w_u32(w, world.objects.len() as u32)?;

    // Camera
    w_f32x3(w, camera.eye)?;
    w_f32x3(w, camera.target)?;
    w_f32x3(w, camera.up)?;
    w_f32(w, camera.aspect)?;
    w_f32(w, camera.fov)?;
    w_f32(w, camera.znear)?;
    w_f32(w, camera.zfar)?;
    w_f32(w, camera.lr_rot)?;
    w_f32(w, camera.ud_rot)?;

    // Roots
    // Store the ordered root list explicitly so load-time order is preserved.
    w_u32(w, world.roots.len() as u32)?;
    for &root_id in &world.roots {
        w_u32(w, root_id as u32)?;
    }

    // Objects (sorted by id for deterministic output)
    let mut ids: Vec<usize> = world.objects.keys().copied().collect();
    ids.sort_unstable();

    for id in ids {
        let obj = &world.objects[&id];

        w_u32(w, id as u32)?;
        w_u32(w, obj.parent.map(|p| p as u32).unwrap_or(NO_PARENT))?;

        let name_bytes = obj.name.as_bytes();
        w_u16(w, name_bytes.len() as u16)?;
        w.write_all(name_bytes)?;

        let bytes = obj.str_id.as_bytes();
        w_u16(w, bytes.len() as u16)?;
        w.write_all(bytes)?;

        w_f32x3(w, obj.transform.position)?;
        w_f32x3(w, obj.transform.rotation)?;
        w_f32x3(w, obj.transform.scale)?;
        w_f32x4(w, obj.color)?;

        write_geometry(w, &obj.geometry)?;

        match &obj.texture_path {
            Some(tp) => {
                let tp_bytes = tp.as_bytes();
                if tp_bytes.len() > u16::MAX as usize {
                    return Err(VtrError::TexturePathTooLong { len: tp_bytes.len() });
                }
                w_u16(w, tp_bytes.len() as u16)?;
                w.write_all(tp_bytes)?;
            }
            None => w_u16(w, 0)?,
        }

        w_u32(w, obj.children.len() as u32)?;
        for &child_id in &obj.children {
            w_u32(w, child_id as u32)?;
        }
    }

    // Text overlay section (V3).
    write_text_overlay(w, overlay)?;

    w.flush()?;
    Ok(())
}

/// Deserialize a complete scene from any [`Read`] source.
/// V2 files produce an empty text overlay; V3 files restore all labels.
pub fn read(r: &mut impl Read) -> Result<SceneData, VtrError> {
    // Header
    let header = read_header(r)?;
    let object_count = header.object_count as usize;

    // Camera
    let camera = Camera {
        eye:    r_f32x3(r)?,
        target: r_f32x3(r)?,
        up:     r_f32x3(r)?,
        aspect: r_f32(r)?,
        fov:    r_f32(r)?,
        znear:  r_f32(r)?,
        zfar:   r_f32(r)?,
        lr_rot: r_f32(r)?,
        ud_rot: r_f32(r)?,
    };

    // Roots
    let roots_count = r_u32(r)? as usize;
    let mut roots = Vec::with_capacity(roots_count);
    for _ in 0..roots_count {
        roots.push(r_u32(r)? as usize);
    }

    // Objects
    let mut objects: HashMap<usize, Object> = HashMap::with_capacity(object_count);
    let mut max_id: usize = 0;

    for _ in 0..object_count {
        let id         = r_u32(r)? as usize;
        let parent_raw = r_u32(r)?;
        let parent     = if parent_raw == NO_PARENT { None } else { Some(parent_raw as usize) };

        let name_len = r_u16(r)? as usize;
        let mut name_bytes = vec![0u8; name_len];
        r.read_exact(&mut name_bytes)?;
        let name = String::from_utf8(name_bytes)?;

        let str_id_len = r_u16(r)? as usize;

        let mut sid_bytes = vec![0u8; str_id_len];
        r.read_exact(&mut sid_bytes)?;
        let str_id = String::from_utf8(sid_bytes)?;

        let position = r_f32x3(r)?;
        let rotation = r_f32x3(r)?;
        let scale = r_f32x3(r)?;
        let color = r_f32x4(r)?;
        let geometry = read_geometry(r)?;

        // texture_path: u16-prefixed UTF-8 string (0 length = no texture)
        let tp_len = r_u16(r)? as usize;
        let texture_path = if tp_len > 0 {
            let mut tp_bytes = vec![0u8; tp_len];
            r.read_exact(&mut tp_bytes)?;
            Some(String::from_utf8(tp_bytes)?)
        } else {
            None
        };

        let children_count = r_u32(r)? as usize;
        let mut children = Vec::with_capacity(children_count);
        for _ in 0..children_count {
            children.push(r_u32(r)? as usize);
        }

        if id > max_id { max_id = id; }

        objects.insert(id, Object {
            name,
            str_id,
            transform: Transform { position, rotation, scale },
            geometry,
            color,
            children,
            parent,
            texture_path,
        });
    }

    let next_id = if objects.is_empty() { 0 } else { max_id + 1 };
    let world   = World::from_parts(objects, roots, next_id);

    // Text overlay: present in V3+, synthesise empty overlay for V2 files.
    let text_overlay = if header.format_version >= 3 {
        read_text_overlay(r, header.format_version)?
    } else {
        TextOverlay { labels: HashMap::new(), next_id: 0, fonts: Vec::new() }
    };

    Ok(SceneData { camera, world, text_overlay })
}

/// Write a scene to a file at the given path, creating or truncating it.
pub fn write_to_file(path: &Path, camera: &Camera, world: &World) -> Result<(), VtrError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write(&mut writer, camera, world)
}

/// Write a scene **with** text overlay to a file, creating or truncating it.
pub fn write_scene_to_file(path: &Path, camera: &Camera, world: &World, overlay: &TextOverlay) -> Result<(), VtrError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    write_scene(&mut writer, camera, world, overlay)
}

/// Read a scene from a `.vtr` file at the given path.
pub fn read_from_file(path: &Path) -> Result<SceneData, VtrError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    read(&mut reader)
}

/// Peek at the header of a `.vtr` file without loading the scene.
pub fn header_from_file(path: &Path) -> Result<VtrHeader, VtrError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    read_header(&mut reader)
}
