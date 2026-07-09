use std::path::Path;

pub struct Triangle {
    pub v0: [f32;4],
    pub v1: [f32;4],
    pub v2: [f32;4],
    /// Packed 0xAARRGGBB surface color from the source material, when the
    /// source carries one; `None` lets the renderer use its own base color.
    pub color: Option<u32>,
}

pub struct Mesh{
    pub triangles: Vec<Triangle>,
}

/// Failure modes shared by the mesh loaders.
#[derive(Debug)]
pub enum MeshError {
    Io(std::io::Error),
    Gltf(String),
    Unsupported(&'static str),
    Empty,
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::Io(e) => write!(f, "io error: {e}"),
            MeshError::Gltf(e) => write!(f, "gltf error: {e}"),
            MeshError::Unsupported(what) => write!(f, "unsupported: {what}"),
            MeshError::Empty => write!(f, "mesh contains no triangles"),
        }
    }
}

impl std::error::Error for MeshError {}

impl From<std::io::Error> for MeshError {
    fn from(e: std::io::Error) -> Self { MeshError::Io(e) }
}

#[cfg(feature = "obj")]
pub fn load_obj<P: AsRef<Path> + std::fmt::Debug>(path: P) -> Mesh {

    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    ).expect("Failed to load OBJ file");

    let mut triangles = Vec::new();

    for model in models{
        let mesh = &model.mesh;

        for chunk in mesh.indices.chunks_exact(3){
            let idx0 = chunk[0] as usize;
            let idx1 = chunk[1] as usize;
            let idx2 = chunk[2] as usize;

            let v0 = [
                mesh.positions[idx0 * 3],
                mesh.positions[idx0 * 3 + 1],
                mesh.positions[idx0 * 3 + 2],
                1.0
            ];

            let v1 =[
                mesh.positions[idx1 * 3],
                mesh.positions[idx1 * 3 + 1],
                mesh.positions[idx1 * 3 + 2],
                1.0
            ];

            let v2 =[
                mesh.positions[idx2 * 3],
                mesh.positions[idx2 * 3 + 1],
                mesh.positions[idx2 * 3 + 2],
                1.0
            ];

            triangles.push(Triangle { v0, v1, v2, color: None });
        }
    }

    Mesh {triangles}

}

#[cfg(feature = "gltf")]
pub fn load_glb_path(path: impl AsRef<Path>) -> Result<Mesh, MeshError> {
    let bytes = std::fs::read(path)?;
    load_glb(&bytes)
}

/// Average color of a material's base-color texture, cached per glTF image
/// index (materials often share textures). `None` when the image is external,
/// undecodable, or fully transparent.
#[cfg(feature = "gltf")]
fn average_image_color(image: gltf::Image, blob: &[u8]) -> Option<[f32; 3]> {
    let gltf::image::Source::View { view, .. } = image.source() else {
        return None;
    };
    if view.buffer().index() != 0 {
        return None;
    }
    let bytes = blob.get(view.offset()..view.offset() + view.length())?;
    let decoded = image::load_from_memory(bytes).ok()?.to_rgba8();

    // Subsample large textures; ~65k samples is plenty for an average.
    let total = (decoded.width() as usize) * (decoded.height() as usize);
    let step = (total / 65536).max(1);
    let mut sum = [0.0f64; 3];
    let mut count = 0.0f64;
    for (i, pixel) in decoded.pixels().enumerate() {
        if i % step != 0 {
            continue;
        }
        let [r, g, b, a] = pixel.0;
        // Transparent texels (atlas padding, decals) would wash out the average.
        if a < 8 {
            continue;
        }
        sum[0] += r as f64;
        sum[1] += g as f64;
        sum[2] += b as f64;
        count += 1.0;
    }
    if count == 0.0 {
        return None;
    }
    Some([
        (sum[0] / count / 255.0) as f32,
        (sum[1] / count / 255.0) as f32,
        (sum[2] / count / 255.0) as f32,
    ])
}

/// Flat color for a primitive: average base-color texture times the material's
/// baseColorFactor. `None` for the default material (no color information),
/// so the renderer's own base color applies.
///
/// Color-space note: texture texels are sRGB and the factor is linear; we mix
/// them directly, which is fine for a low-resolution preview.
#[cfg(feature = "gltf")]
fn primitive_base_color(
    primitive: &gltf::Primitive,
    blob: &[u8],
    image_averages: &mut std::collections::HashMap<usize, Option<[f32; 3]>>,
) -> Option<u32> {
    let pbr = primitive.material().pbr_metallic_roughness();
    let factor = pbr.base_color_factor();
    let texture_average = pbr.base_color_texture().and_then(|info| {
        let image = info.texture().source();
        *image_averages
            .entry(image.index())
            .or_insert_with(|| average_image_color(image, blob))
    });

    let rgb = match texture_average {
        Some(avg) => [avg[0] * factor[0], avg[1] * factor[1], avg[2] * factor[2]],
        None if factor == [1.0, 1.0, 1.0, 1.0] => return None,
        None => [factor[0], factor[1], factor[2]],
    };

    let channel = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(0xFF000000 | (channel(rgb[0]) << 16) | (channel(rgb[1]) << 8) | channel(rgb[2]))
}

/// Load a binary glTF (.glb) with its buffer embedded in the BIN chunk.
/// Positions plus a flat per-primitive material color; normals/UVs are
/// ignored.
#[cfg(feature = "gltf")]
pub fn load_glb(bytes: &[u8]) -> Result<Mesh, MeshError> {
    // Skip validation: we only read positions, and strict validation rejects
    // otherwise-fine files over features we never touch (e.g. a required
    // EXT_texture_webp texture extension). Malformed containers/JSON still
    // error out of the parse itself.
    let gltf = gltf::Gltf::from_slice_without_validation(bytes)
        .map_err(|e| MeshError::Gltf(e.to_string()))?;
    let Some(blob) = gltf.blob.as_deref() else {
        return Err(MeshError::Unsupported("glTF without an embedded BIN chunk (external buffers)"));
    };
    let document = &gltf.document;

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or(MeshError::Unsupported("glTF with no scenes"))?;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    // Composes row-major transforms: world = parent · local.
    fn mat_mul_mat(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
        let mut out = [[0.0f32; 4]; 4];
        for row in 0..4 {
            for col in 0..4 {
                out[row][col] = (0..4).map(|k| a[row][k] * b[k][col]).sum();
            }
        }
        out
    }

    fn collect_node(
        node: gltf::Node,
        parent: &[[f32; 4]; 4],
        blob: &[u8],
        triangles: &mut Vec<Triangle>,
        image_averages: &mut std::collections::HashMap<usize, Option<[f32; 3]>>,
    ) {
        // gltf returns column-major matrices; mat_mul is row-major — transpose.
        let local_cm = node.transform().matrix();
        let mut local = [[0.0f32; 4]; 4];
        for row in 0..4 {
            for col in 0..4 {
                local[row][col] = local_cm[col][row];
            }
        }
        let world = mat_mul_mat(parent, &local);

        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    continue;
                }
                let reader = primitive.reader(|buffer| match buffer.source() {
                    gltf::buffer::Source::Bin => Some(blob),
                    gltf::buffer::Source::Uri(_) => None,
                });
                let Some(positions) = reader.read_positions() else { continue; };
                let positions: Vec<[f32; 3]> = positions.collect();
                let color = primitive_base_color(&primitive, blob, image_averages);

                let transformed = |idx: usize| -> Option<[f32; 4]> {
                    let p = positions.get(idx)?;
                    Some(crate::calculations::mat_mul(&world, [p[0], p[1], p[2], 1.0]))
                };

                match reader.read_indices() {
                    Some(indices) => {
                        let indices: Vec<u32> = indices.into_u32().collect();
                        for chunk in indices.chunks_exact(3) {
                            if let (Some(v0), Some(v1), Some(v2)) = (
                                transformed(chunk[0] as usize),
                                transformed(chunk[1] as usize),
                                transformed(chunk[2] as usize),
                            ) {
                                triangles.push(Triangle { v0, v1, v2, color });
                            }
                        }
                    }
                    None => {
                        for chunk_start in (0..positions.len() / 3 * 3).step_by(3) {
                            if let (Some(v0), Some(v1), Some(v2)) = (
                                transformed(chunk_start),
                                transformed(chunk_start + 1),
                                transformed(chunk_start + 2),
                            ) {
                                triangles.push(Triangle { v0, v1, v2, color });
                            }
                        }
                    }
                }
            }
        }

        for child in node.children() {
            collect_node(child, &world, blob, triangles, image_averages);
        }
    }

    let mut triangles = Vec::new();
    let mut image_averages = std::collections::HashMap::new();
    for node in scene.nodes() {
        collect_node(node, &IDENTITY, blob, &mut triangles, &mut image_averages);
    }

    if triangles.is_empty() {
        return Err(MeshError::Empty);
    }
    Ok(Mesh { triangles })
}

#[cfg(all(test, feature = "gltf"))]
mod gltf_tests {
    use super::*;

    /// Assemble a GLB container: 12-byte header, JSON chunk (space-padded),
    /// BIN chunk (zero-padded).
    fn build_glb(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json_bytes = json.as_bytes().to_vec();
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let mut bin_bytes = bin.to_vec();
        while bin_bytes.len() % 4 != 0 {
            bin_bytes.push(0);
        }

        let total = 12 + 8 + json_bytes.len() + 8 + bin_bytes.len();
        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin_bytes);
        glb
    }

    fn triangle_glb(node_extra: &str) -> Vec<u8> {
        let positions: [f32; 9] = [
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        let bin: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"mesh": 0{node_extra}}}],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "accessors": [{{
                    "bufferView": 0, "componentType": 5126, "count": 3,
                    "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]
                }}],
                "bufferViews": [{{"buffer": 0, "byteOffset": 0, "byteLength": 36}}],
                "buffers": [{{"byteLength": 36}}]
            }}"#
        );
        build_glb(&json, &bin)
    }

    #[test]
    fn load_glb_parses_minimal_blob() {
        let mesh = load_glb(&triangle_glb("")).unwrap();
        assert_eq!(mesh.triangles.len(), 1);
        let t = &mesh.triangles[0];
        assert_eq!(t.v0, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.v1, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(t.v2, [0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn load_glb_applies_node_transform_row_major() {
        // A translated node: if the column-major gltf matrix were fed to the
        // row-major pipeline untransposed, the translation would land in the
        // wrong components.
        let mesh = load_glb(&triangle_glb(r#", "translation": [1.0, 2.0, 3.0]"#)).unwrap();
        let t = &mesh.triangles[0];
        assert_eq!(t.v0, [1.0, 2.0, 3.0, 1.0]);
        assert_eq!(t.v1, [2.0, 2.0, 3.0, 1.0]);
        assert_eq!(t.v2, [1.0, 3.0, 3.0, 1.0]);
    }

    #[test]
    fn load_glb_rejects_garbage() {
        assert!(load_glb(b"not a glb").is_err());
    }

    #[test]
    fn load_glb_reads_base_color_factor() {
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bin: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
        let json = r#"{
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "materials": [{"pbrMetallicRoughness": {"baseColorFactor": [1.0, 0.0, 0.0, 1.0]}}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "material": 0}]}],
            "accessors": [{
                "bufferView": 0, "componentType": 5126, "count": 3,
                "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]
            }],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
            "buffers": [{"byteLength": 36}]
        }"#;
        let mesh = load_glb(&build_glb(json, &bin)).unwrap();
        assert_eq!(mesh.triangles[0].color, Some(0xFFFF0000));
    }

    #[test]
    fn load_glb_default_material_has_no_color() {
        let mesh = load_glb(&triangle_glb("")).unwrap();
        assert_eq!(mesh.triangles[0].color, None);
    }

    #[test]
    fn load_glb_averages_base_color_texture() {
        // 2x2 all-green PNG, encoded in memory.
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 200, 0, 255]))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let png = png.into_inner();

        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
        let png_offset = bin.len();
        bin.extend_from_slice(&png);

        let json = format!(
            r#"{{
            "asset": {{"version": "2.0"}},
            "scene": 0,
            "scenes": [{{"nodes": [0]}}],
            "nodes": [{{"mesh": 0}}],
            "images": [{{"bufferView": 1, "mimeType": "image/png"}}],
            "textures": [{{"source": 0}}],
            "materials": [{{"pbrMetallicRoughness": {{"baseColorTexture": {{"index": 0}}}}}}],
            "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}, "material": 0}}]}}],
            "accessors": [{{
                "bufferView": 0, "componentType": 5126, "count": 3,
                "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]
            }}],
            "bufferViews": [
                {{"buffer": 0, "byteOffset": 0, "byteLength": 36}},
                {{"buffer": 0, "byteOffset": {png_offset}, "byteLength": {png_len}}}
            ],
            "buffers": [{{"byteLength": {total_len}}}]
        }}"#,
            png_len = png.len(),
            total_len = bin.len(),
        );
        let mesh = load_glb(&build_glb(&json, &bin)).unwrap();
        assert_eq!(mesh.triangles[0].color, Some(0xFF00C800));
    }

    #[test]
    fn load_glb_tolerates_unsupported_required_extensions() {
        // Real generated models require texture extensions (EXT_texture_webp)
        // that the positions-only loader never touches; they must still load.
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bin: Vec<u8> = positions.iter().flat_map(|f| f.to_le_bytes()).collect();
        let json = r#"{
            "asset": {"version": "2.0"},
            "extensionsUsed": ["EXT_texture_webp"],
            "extensionsRequired": ["EXT_texture_webp"],
            "scene": 0,
            "scenes": [{"nodes": [0]}],
            "nodes": [{"mesh": 0}],
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
            "accessors": [{
                "bufferView": 0, "componentType": 5126, "count": 3,
                "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0]
            }],
            "bufferViews": [{"buffer": 0, "byteOffset": 0, "byteLength": 36}],
            "buffers": [{"byteLength": 36}]
        }"#;
        let mesh = load_glb(&build_glb(json, &bin)).unwrap();
        assert_eq!(mesh.triangles.len(), 1);
    }

    #[test]
    fn load_glb_empty_scene_is_empty_error() {
        let json = r#"{
            "asset": {"version": "2.0"},
            "scene": 0,
            "scenes": [{"nodes": []}],
            "buffers": [{"byteLength": 4}]
        }"#;
        let glb = build_glb(json, &[0, 0, 0, 0]);
        assert!(matches!(load_glb(&glb), Err(MeshError::Empty)));
    }
}
