//! Load a GLB and report triangle count and distinct material colors.
//! Usage: cargo run --example glb_probe --features gltf -- <path.glb>

fn main() {
    let path = std::env::args().nth(1).expect("usage: glb_probe <path.glb>");
    let mesh = rasterizer::io::load_glb_path(&path).expect("failed to load GLB");

    let mut colors = std::collections::BTreeMap::new();
    for triangle in &mesh.triangles {
        *colors.entry(triangle.color).or_insert(0usize) += 1;
    }

    println!("triangles: {}", mesh.triangles.len());
    println!("distinct colors: {}", colors.len());
    for (color, count) in colors {
        match color {
            Some(c) => println!("  #{:06X}: {count} triangles", c & 0xFFFFFF),
            None => println!("  (no material color): {count} triangles"),
        }
    }
}
