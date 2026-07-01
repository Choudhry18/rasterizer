use std::fmt::Debug;
use std::path::Path;
use tobj;

pub struct Triangle {
    pub v0: [f32; 4],
    pub v1: [f32; 4],
    pub v2: [f32; 4],
}

pub struct Mesh {
    pub triangles: Vec<Triangle>,
}

pub fn try_load_mesh_as_ndarray<P: AsRef<Path> + Debug>(path: P) -> Result<Mesh, tobj::LoadError> {
    let (models, _materials) = tobj::load_obj(
        path,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
    )?;

    let mut triangles = Vec::new();

    for model in models {
        let mesh = &model.mesh;

        for chunk in mesh.indices.chunks_exact(3) {
            let idx0 = chunk[0] as usize;
            let idx1 = chunk[1] as usize;
            let idx2 = chunk[2] as usize;

            let v0 = [
                mesh.positions[idx0 * 3],
                mesh.positions[idx0 * 3 + 1],
                mesh.positions[idx0 * 3 + 2],
                1.0,
            ];

            let v1 = [
                mesh.positions[idx1 * 3],
                mesh.positions[idx1 * 3 + 1],
                mesh.positions[idx1 * 3 + 2],
                1.0,
            ];

            let v2 = [
                mesh.positions[idx2 * 3],
                mesh.positions[idx2 * 3 + 1],
                mesh.positions[idx2 * 3 + 2],
                1.0,
            ];

            triangles.push(Triangle { v0, v1, v2 });
        }
    }

    Ok(Mesh { triangles })
}

pub fn load_mesh_as_ndarray<P: AsRef<Path> + Debug>(path: P) -> Mesh {
    try_load_mesh_as_ndarray(path).expect("Failed to load OBJ file")
}
