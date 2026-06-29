//! Orbit camera: builds the **view matrix** (world → camera) and the controls
//! (pan / orbit / dolly) that drive it.
//!
//! The camera is stored as a target (`cam_to`), an orbit `radius`, and two
//! angles (`yaw`, `pitch`) rather than an explicit eye position; the eye is
//! recovered on demand by [`CameraState::cam_from`].
//!
//! This is a **left-handed** look-at: `forward` points toward the target along
//! +Z, and `right = up × forward`. That handedness is why the projection maps
//! onto +Z and the near-plane cull tests `z <= NEAR`.
use crate::calculations::{subtract, normalize, dot_product, cross_product, add};
pub struct CameraState {
    cam_to: [f32; 3],
    cam_up: [f32; 3],
    yaw:    f32,
    pitch:  f32,
    radius: f32,
}

impl CameraState {
    pub fn new(cam_to: [f32; 3], cam_up: [f32; 3], yaw: f32, pitch: f32, radius: f32) -> Self {
        Self { cam_to, cam_up, yaw, pitch, radius }
    }

    /// Recover the eye (camera) position from the orbit parameters using
    /// spherical coordinates around the target.
    ///
    /// ```text
    /// eye = target + radius · ( cos(pitch)·sin(yaw),
    ///                           sin(pitch),
    ///                           cos(pitch)·cos(yaw) )
    /// ```
    ///
    /// `yaw` rotates around world-up (Y), `pitch` raises/lowers the eye, and
    /// `radius` is the orbit distance.
    ///
    /// The eye sits *exactly* `radius` from the target because the distance
    /// collapses via the Pythagorean identity (`sin²θ + cos²θ = 1`) applied
    /// twice:
    ///
    /// ```text
    /// d² = r²[ cos²p·sin²y + sin²p + cos²p·cos²y ]
    ///    = r²[ cos²p·(sin²y + cos²y) + sin²p ]   ← identity on yaw   → (…) = 1
    ///    = r²[ cos²p + sin²p ]                    ← identity on pitch → (…) = 1
    ///    = r²
    /// ```
    ///
    /// That invariant is what lets [`CameraState::orbit`] (changes the angles)
    /// and [`CameraState::dolly`] (changes `radius`) keep the camera on a
    /// perfect sphere around the target.
    fn cam_from(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        [
            self.cam_to[0] + self.radius * cp * self.yaw.sin(),
            self.cam_to[1] + self.radius * self.pitch.sin(),
            self.cam_to[2] + self.radius * cp * self.yaw.cos(),
        ]
    }

    /// Build the view matrix that maps world space into camera space.
    ///
    /// First an orthonormal camera basis is derived from the eye/target:
    ///
    /// ```text
    /// forward = normalize(target − eye)
    /// right   = normalize(worldUp × forward)
    /// up      = forward × right
    /// ```
    ///
    /// `right` and `up` are re-derived via cross products so the basis stays
    /// orthonormal even if `worldUp` isn't exactly perpendicular to `forward`.
    ///
    /// The view matrix is the **inverse** of the camera's world placement. For
    /// an orthonormal rotation the inverse is the transpose, and the translation
    /// becomes the negative dot of each axis with the eye:
    ///
    /// ```text
    /// V = [ rightₓ  right_y  right_z  −(right·eye) ]
    ///     [ upₓ     up_y     up_z     −(up·eye)    ]
    ///     [ fwdₓ    fwd_y    fwd_z    −(fwd·eye)   ]
    ///     [ 0       0        0         1           ]
    /// ```
    pub fn build_view(&self) -> [[f32; 4]; 4] {
        let cam_from = self.cam_from();
        let forward  = normalize(subtract(self.cam_to, cam_from));
        let right    = normalize(cross_product(self.cam_up, forward));
        let newup    = cross_product(forward, right);
        let tx = -dot_product(right,   cam_from);
        let ty = -dot_product(newup,   cam_from);
        let tz = -dot_product(forward, cam_from);

        [
            [right[0],   right[1],   right[2],   tx],
            [newup[0],   newup[1],   newup[2],   ty],
            [forward[0], forward[1], forward[2], tz],
            [0.0,        0.0,        0.0,        1.0],
        ]
    }

    /// Slide the orbit target within the camera's view plane.
    ///
    /// Rebuilds the `right`/`up` basis and moves the target along it:
    ///
    /// ```text
    /// target ← target + right·Δright + up·Δup
    /// ```
    pub fn pan(&mut self, right_amount: f32, up_amount: f32) {
        let cam_from = self.cam_from();
        let forward  = normalize(subtract(self.cam_to, cam_from));
        let right    = normalize(cross_product(self.cam_up, forward));
        let up       = cross_product(forward, right);

        let offset = [
            right[0] * right_amount + up[0] * up_amount,
            right[1] * right_amount + up[1] * up_amount,
            right[2] * right_amount + up[2] * up_amount,
        ];

        self.cam_to = add(self.cam_to, offset);
    }

    /// Rotate the eye around the target by adjusting the spherical angles.
    ///
    /// Pitch is clamped to avoid flipping over the poles:
    ///
    /// ```text
    /// yaw   ← yaw + Δyaw
    /// pitch ← clamp(pitch + Δpitch, −1.4, +1.4)
    /// ```
    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        const MAX_PITCH: f32 = 1.4;
        self.yaw   += yaw;
        self.pitch  = (self.pitch + pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Move the eye toward/away from the target by changing the orbit radius,
    /// floored so it can't cross the target.
    ///
    /// ```text
    /// radius ← max(radius − amount, 0.1)
    /// ```
    pub fn dolly(&mut self, amount: f32) {
        self.radius = (self.radius - amount).max(0.1);
    }
}
