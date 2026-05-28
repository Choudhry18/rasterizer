# Framebuffer

Owns color buffers, depth buffers, pixel writes, clears, and output-facing image
data.

The z-buffer starts here as depth storage plus depth-test behavior. Split into a
dedicated `depth` module only once depth behavior becomes substantial.

