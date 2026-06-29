//! Vector and matrix primitives used across the rasterizer.
//!
//! Conventions:
//! - Vectors are **column vectors**; matrices are stored **row-major** as `[[f32;4];4]`.
//! - A point is transformed as `M · v` (matrix on the left) via [`mat_mul`].
//! - Homogeneous coordinates: a 3D point `(x, y, z)` is carried as `(x, y, z, 1)`,
//!   a direction as `(x, y, z, 0)`.

/// Component-wise vector sum.
///
/// ```text
/// a + b = (aₓ+bₓ, a_y+b_y, a_z+b_z)
/// ```
///
/// Used to translate the orbit target during a pan.
pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Component-wise vector difference.
///
/// ```text
/// a − b = (aₓ−bₓ, a_y−b_y, a_z−b_z)
/// ```
///
/// Produces direction vectors (e.g. `target − eye`, or the edges of a triangle).
pub fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Dot (scalar) product — the scalar projection of one vector onto another.
///
/// ```text
/// a · b = aₓbₓ + a_y b_y + a_z b_z = |a||b|cos θ
/// ```
///
/// Used for the length-squared inside [`normalize`] (`a · a = |a|²`), the
/// view-matrix translation terms, and the diffuse lighting term (`N · L`).
pub fn dot_product(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Scale a vector to unit length so it represents a pure direction.
///
/// ```text
/// â = a / |a|,   where |a| = √(a · a)
/// ```
pub fn normalize(a: [f32; 3]) -> [f32; 3] {
    let magnitude = dot_product(a, a).sqrt();
    a.map(|x| x / magnitude)
}

/// Cross product — a vector perpendicular to both `a` and `b`.
///
/// ```text
/// a × b = ( a_y b_z − a_z b_y,
///           a_z bₓ − aₓ b_z,
///           aₓ b_y − a_y bₓ )
/// ```
///
/// Its length is `|a||b|sin θ`. Used to build the camera basis (right/up) and
/// the triangle surface normal.
pub fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// 4×4 matrix times a 4-vector — the core transform operation.
///
/// ```text
/// (M · v)_i = Σⱼ Mᵢⱼ vⱼ
/// ```
///
/// Each output component is the dot product of a matrix row with the input
/// vector. Every stage of the vertex pipeline (model → world → camera → clip)
/// is one call to this.
pub fn mat_mul(a: &[[f32;4];4],b: [f32;4]) -> [f32;4]{
    let row1 = a[0][0] * b[0] + a[0][1] * b[1] + a[0][2] * b[2] + a[0][3] * b[3];
    let row2 = a[1][0] * b[0] + a[1][1] * b[1] + a[1][2] * b[2] + a[1][3] * b[3];
    let row3 = a[2][0] * b[0] + a[2][1] * b[1] + a[2][2] * b[2] + a[2][3] * b[3];
    let row4 = a[3][0] * b[0] + a[3][1] * b[1] + a[3][2] * b[2] + a[3][3] * b[3];

    [row1, row2, row3, row4]
}