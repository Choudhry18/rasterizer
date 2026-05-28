# Rasterizer Architecture

## Dependency Direction

Keep dependencies flowing from high-level application code toward low-level
rendering primitives:

```text
app
  -> scene and io
  -> pipeline
  -> raster and shader
  -> geometry, math, framebuffer
```

Low-level modules should not depend on CLI parsing, file formats, or application
configuration.

## Module Responsibilities

`src/app`

Command-line handling, render configuration, and executable setup. This layer
should stay thin.

`src/math`

Vectors, matrices, transforms, projection helpers, and numeric utilities.

`src/geometry`

Vertices, triangles, meshes, attributes, and model-space structures.

`src/scene`

Cameras, lights, materials, model instances, and scene composition.

`src/framebuffer`

Color buffer, depth buffer, clearing, pixel writes, and output-facing image data.
The z-buffer starts here as depth-buffer storage and depth-test behavior.

`src/pipeline`

The high-level render flow: vertex processing, clipping, projection, viewport
mapping, rasterization dispatch, fragment shading, and framebuffer writes.

`src/raster`

Triangle coverage, edge functions, barycentric coordinates, interpolation, and
screen-space rasterization rules.

`src/shader`

Small shading strategies such as flat color, interpolated vertex color, depth
visualization, normals, wireframe, and later texture sampling.

`src/io`

Asset loading and file output. OBJ loading, image writing, and scene import live
here rather than inside the renderer core.

## Growth Order

1. Draw one hardcoded triangle into a color buffer.
2. Add a depth buffer.
3. Add transforms and a camera.
4. Add barycentric interpolation.
5. Add simple shading modes.
6. Add mesh loading.
7. Add clipping.
8. Add textures and materials.
9. Add golden-image tests.
10. Add benchmarks after behavior is stable.

