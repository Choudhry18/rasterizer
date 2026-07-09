//! Software rasterizer: mesh loading, an orbit camera, and a headless
//! render-one-frame pipeline that draws into caller-provided buffers.
//!
//! Feature flags:
//! - `window` — the interactive winit/softbuffer viewer binary.
//! - `obj` — OBJ loading via `tobj` ([`io::load_obj`]).
//! - `gltf` — GLB loading ([`io::load_glb`]).

pub mod calculations;
pub mod camera;
pub mod io;
pub mod raster;
pub mod render;
