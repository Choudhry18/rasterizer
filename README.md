# Rasterizer

A Rust software rasterizer project.

The repository is organized as a renderer library with a thin executable entry
point. The current scaffold is intentionally light on implementation and focuses
on keeping rendering responsibilities separated from CLI, file IO, and output.

## Project Layout

```text
src/
  main.rs
  app/
  math/
  geometry/
  scene/
  framebuffer/
  pipeline/
  raster/
  shader/
  io/
examples/
assets/
tests/
benches/
docs/
```

See `docs/architecture.md` for the module blueprint.

