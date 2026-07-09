//! Headless one-frame rendering: the vertex pipeline (model → world → camera →
//! clip → screen) plus Lambert shading, drawing into caller-provided color and
//! depth buffers. This is the same pipeline the window binary runs per frame,
//! extracted so any consumer (e.g. a terminal front-end) can rasterize a mesh
//! at an arbitrary resolution.

use crate::calculations::{cross_product, dot_product, mat_mul, normalize, subtract};
use crate::camera::CameraState;
use crate::io::Mesh;
use crate::raster;

/// Axis-aligned bounding box of a mesh in model space.
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Bounding box over every triangle vertex; `None` for an empty mesh.
pub fn mesh_bounds(mesh: &Mesh) -> Option<Bounds> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let mut any = false;

    for triangle in mesh.triangles.iter() {
        for v in [triangle.v0, triangle.v1, triangle.v2] {
            any = true;
            for axis in 0..3 {
                min[axis] = min[axis].min(v[axis]);
                max[axis] = max[axis].max(v[axis]);
            }
        }
    }

    any.then_some(Bounds { min, max })
}

/// Per-frame render settings. `Default` matches the window binary's constants.
pub struct FrameOptions {
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
    /// Height of one output pixel relative to its width; 1.0 = square pixels.
    /// Terminal character cells are roughly twice as tall as wide, so a
    /// terminal front-end passes ~1.8 to undo the vertical stretch.
    pub cell_aspect: f32,
    /// Packed 0xAARRGGBB clear color.
    pub background: u32,
    /// Packed 0xAARRGGBB surface color before shading.
    pub base_color: u32,
    pub light_dir: [f32; 3],
    pub ambient: f32,
}

impl Default for FrameOptions {
    fn default() -> Self {
        Self {
            fov_y: std::f32::consts::FRAC_PI_3,
            near: 0.1,
            far: 1000.0,
            cell_aspect: 1.0,
            background: 0xFF_1A_1A_2E,
            base_color: 0xFF_B6_C1,
            light_dir: [0.3, -0.8, 0.5],
            ambient: 0.2,
        }
    }
}

/// Perspective projection matrix (camera → clip space).
///
/// Left-handed, zero-to-one-depth (D3D-style). `f_term = 1/tan(fov_y/2)` is
/// the focal length; dividing by aspect undoes non-square framebuffers, row 3
/// remaps depth to [0, 1], and row 4 copies camera-space `z` into `w` for the
/// perspective divide.
pub fn build_projection(fov_y: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let d = far / (far - near);
    [
        [f / aspect, 0.0, 0.0,        0.0],
        [0.0,        f,   0.0,        0.0],
        [0.0,        0.0, d,         -near * d],
        [0.0,        0.0, 1.0,        0.0],
    ]
}

/// Perspective divide + viewport transform (clip → NDC → screen pixels).
/// `y` is flipped because screen rows grow downward; `ndc_z` passes through
/// for depth testing.
pub fn to_screen(v_clip: [f32; 4], width: f32, height: f32) -> [f32; 3] {
    let inv_w = 1.0 / v_clip[3];
    let ndc_x = v_clip[0] * inv_w;
    let ndc_y = v_clip[1] * inv_w;
    let ndc_z = v_clip[2] * inv_w;

    let px = (ndc_x + 1.0) * 0.5 * width;
    let py = (1.0 - ndc_y) * 0.5 * height;

    [px, py, ndc_z]
}

/// Scale a packed `0xAARRGGBB` color's RGB channels by a lighting `intensity`.
pub fn shade(color: u32, intensity: f32) -> u32 {
    let r = (((color >> 16) & 0xFF) as f32 * intensity) as u32 & 0xFF;
    let g = (((color >>  8) & 0xFF) as f32 * intensity) as u32 & 0xFF;
    let b = (( color        & 0xFF) as f32 * intensity) as u32 & 0xFF;
    0xFF000000 | (r << 16) | (g << 8) | b
}

/// Render one frame of `mesh` into `color`/`depth` (each `width * height`
/// long). Clears both buffers first, then runs the full pipeline per triangle:
/// model → world → camera (near-cull) → clip → screen, flat Lambert shading,
/// z-buffered fill.
///
/// # Panics
/// Panics if `color` or `depth` is shorter than `width * height`.
pub fn render_frame(
    mesh: &Mesh,
    model: &[[f32; 4]; 4],
    camera: &CameraState,
    opts: &FrameOptions,
    width: usize,
    height: usize,
    color: &mut [u32],
    depth: &mut [f32],
) {
    assert!(color.len() >= width * height && depth.len() >= width * height);

    color[..width * height].fill(opts.background);
    depth[..width * height].fill(f32::INFINITY);

    if width == 0 || height == 0 {
        return;
    }

    let w = width as f32;
    let h = height as f32;
    // cell_aspect folds the output medium's pixel shape into the projection:
    // a taller-than-wide cell shrinks the effective height.
    let aspect = w / (h * opts.cell_aspect);

    let view = camera.build_view();
    let proj = build_projection(opts.fov_y, aspect, opts.near, opts.far);

    let light = normalize(opts.light_dir);
    let neg_light = [-light[0], -light[1], -light[2]];

    for triangle in mesh.triangles.iter() {
        let w0 = mat_mul(model, triangle.v0);
        let w1 = mat_mul(model, triangle.v1);
        let w2 = mat_mul(model, triangle.v2);

        let v0_cam = mat_mul(&view, w0);
        let v1_cam = mat_mul(&view, w1);
        let v2_cam = mat_mul(&view, w2);

        // Near-plane cull (left-handed: camera looks down +Z).
        if v0_cam[2] <= opts.near || v1_cam[2] <= opts.near || v2_cam[2] <= opts.near {
            continue;
        }

        let v0_clip = mat_mul(&proj, v0_cam);
        let v1_clip = mat_mul(&proj, v1_cam);
        let v2_clip = mat_mul(&proj, v2_cam);

        let p0 = to_screen(v0_clip, w, h);
        let p1 = to_screen(v1_clip, w, h);
        let p2 = to_screen(v2_clip, w, h);

        let normal = normalize(cross_product(
            subtract(w1[..3].try_into().unwrap(), w0[..3].try_into().unwrap()),
            subtract(w2[..3].try_into().unwrap(), w0[..3].try_into().unwrap()),
        ));
        // Degenerate (zero-area) triangles normalize to NaN — skip them.
        if !normal.iter().all(|c| c.is_finite()) {
            continue;
        }

        let diffuse = dot_product(normal, neg_light).max(0.0);
        let intensity = opts.ambient + (1.0 - opts.ambient) * diffuse;
        let color_shaded = shade(opts.base_color, intensity);

        raster::draw_triangle(color, depth, width, height, p0, p1, p2, color_shaded);
    }
}

pub const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// Build an orbit camera that frames `bounds` fully in view.
///
/// The target is the bounds' center and the orbit radius is chosen so the
/// bounding sphere fits inside the narrower of the vertical/horizontal FOV:
/// `distance = margin · r / sin(min(fov_y, fov_x) / 2)`.
pub fn frame_camera(
    bounds: &Bounds,
    fov_y: f32,
    aspect: f32,
    margin: f32,
    yaw: f32,
    pitch: f32,
) -> CameraState {
    let center = [
        (bounds.min[0] + bounds.max[0]) * 0.5,
        (bounds.min[1] + bounds.max[1]) * 0.5,
        (bounds.min[2] + bounds.max[2]) * 0.5,
    ];
    let extent = subtract(bounds.max, bounds.min);
    let r = (dot_product(extent, extent).sqrt() * 0.5).max(1e-6);

    let fov_x = 2.0 * ((fov_y * 0.5).tan() * aspect).atan();
    let half_fov = (fov_y.min(fov_x) * 0.5).max(1e-3);
    let distance = margin * r / half_fov.sin();

    CameraState::new(center, [0.0, 1.0, 0.0], yaw, pitch, distance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{Mesh, Triangle};

    fn triangle_mesh() -> Mesh {
        Mesh {
            triangles: vec![Triangle {
                v0: [-1.0, -1.0, 0.0, 1.0],
                v1: [1.0, -1.0, 0.0, 1.0],
                v2: [0.0, 1.0, 0.0, 1.0],
            }],
        }
    }

    #[test]
    fn mesh_bounds_of_known_triangle() {
        let bounds = mesh_bounds(&triangle_mesh()).unwrap();
        assert_eq!(bounds.min, [-1.0, -1.0, 0.0]);
        assert_eq!(bounds.max, [1.0, 1.0, 0.0]);
    }

    #[test]
    fn mesh_bounds_empty_is_none() {
        assert!(mesh_bounds(&Mesh { triangles: vec![] }).is_none());
    }

    #[test]
    fn render_frame_draws_visible_pixels() {
        let mesh = triangle_mesh();
        let bounds = mesh_bounds(&mesh).unwrap();
        let opts = FrameOptions::default();
        let (width, height) = (64usize, 48usize);
        let aspect = width as f32 / (height as f32 * opts.cell_aspect);
        let camera = frame_camera(&bounds, opts.fov_y, aspect, 1.15, std::f32::consts::PI, 0.0);

        let mut color = vec![0u32; width * height];
        let mut depth = vec![0f32; width * height];
        render_frame(&mesh, &IDENTITY, &camera, &opts, width, height, &mut color, &mut depth);

        assert!(color.iter().any(|px| *px != opts.background));
    }

    #[test]
    fn frame_camera_keeps_model_inside_viewport() {
        let mesh = triangle_mesh();
        let bounds = mesh_bounds(&mesh).unwrap();
        let opts = FrameOptions::default();
        let (width, height) = (64usize, 48usize);
        let aspect = width as f32 / height as f32;
        let camera = frame_camera(&bounds, opts.fov_y, aspect, 1.15, std::f32::consts::PI, 0.3);

        let mut color = vec![0u32; width * height];
        let mut depth = vec![0f32; width * height];
        render_frame(&mesh, &IDENTITY, &camera, &opts, width, height, &mut color, &mut depth);

        // No lit pixel may touch the border row/column — the silhouette is not clipped.
        for y in 0..height {
            for x in 0..width {
                if color[y * width + x] != opts.background {
                    assert!(x > 0 && x < width - 1 && y > 0 && y < height - 1,
                        "lit pixel on viewport border at ({x}, {y})");
                }
            }
        }
    }

    #[test]
    fn frame_camera_scales_with_bbox() {
        let small = Bounds { min: [-1.0; 3], max: [1.0; 3] };
        let large = Bounds { min: [-10.0; 3], max: [10.0; 3] };
        let fov = std::f32::consts::FRAC_PI_3;

        let d_small = frame_camera(&small, fov, 1.0, 1.15, 0.0, 0.0).build_view();
        let d_large = frame_camera(&large, fov, 1.0, 1.15, 0.0, 0.0).build_view();
        // The view translation's z magnitude reflects the orbit distance.
        let z_small = d_small[2][3].abs();
        let z_large = d_large[2][3].abs();
        assert!((z_large / z_small - 10.0).abs() < 0.1, "expected ~10x distance, got {}", z_large / z_small);
    }

    #[test]
    fn render_frame_skips_degenerate_triangles() {
        let mesh = Mesh {
            triangles: vec![Triangle {
                v0: [0.0, 0.0, 0.0, 1.0],
                v1: [0.0, 0.0, 0.0, 1.0],
                v2: [0.0, 0.0, 0.0, 1.0],
            }],
        };
        let opts = FrameOptions::default();
        let camera = CameraState::new([0.0; 3], [0.0, 1.0, 0.0], std::f32::consts::PI, 0.0, 5.0);
        let mut color = vec![0u32; 16 * 16];
        let mut depth = vec![0f32; 16 * 16];
        render_frame(&mesh, &IDENTITY, &camera, &opts, 16, 16, &mut color, &mut depth);
        assert!(color.iter().all(|px| *px == opts.background));
    }
}
