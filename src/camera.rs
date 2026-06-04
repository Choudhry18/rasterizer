pub struct CameraState {
    cam_from: [f32; 3],
    cam_to:   [f32; 3],
    cam_up:   [f32; 3],
}

impl CameraState {
    pub fn new(cam_from: [f32; 3], cam_to: [f32; 3], cam_up: [f32; 3]) -> Self {
        Self { cam_from, cam_to, cam_up }
    }
    pub fn build_view(&self) -> [[f32;4];4]{
        // rotation three orthonormal vectors expressing the camera in the world space
        let forward = normalize(subtract(self.cam_to,self.cam_from));
        let right = normalize(cross_product(self.cam_up, forward));
        let newup = cross_product(forward, right);
        let tx = -1.0 * dot_product(right, self.cam_from);
        let ty = -1.0 * dot_product(newup, self.cam_from);i 
        let tz = -1.0 * dot_product(forward, self.cam_from);

        //inverse transaltion
        [[right[0], right[1], right[2], tx], [newup[0], newup[1], newup[2], ty], [forward[0], forward[1], forward[2], tz], [0.0, 0.0, 0.0, 1.0]]
    }
    pub fn pan(&mut self, dx: f32, dy: f32) {
    let forward = normalize(subtract(self.cam_to, self.cam_from));
    let right = normalize(cross_product(self.cam_up, forward));
    let up = cross_product(forward, right);

    let scale = 0.01;

    let offset = [
        (-right[0] * dx + up[0] * dy) * scale,
        (-right[1] * dx + up[1] * dy) * scale,
        (-right[2] * dx + up[2] * dy) * scale,
    ];

    self.cam_from = add(self.cam_from, offset);
    self.cam_to = add(self.cam_to, offset);
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32;3]{
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32;3]{
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot_product(a: [f32; 3], b: [f32; 3]) -> f32{
    (a[0] * b[0]) + (a[1] * b[1]) + (a[2] * b[2])
}

fn normalize(a: [f32; 3]) -> [f32; 3]{
    let magnitude = dot_product(a, a).sqrt();
    a.map(|x| x/magnitude)
}

fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32;3]{
    [(a[1] * b[2]) - (a[2] * b[1]), (a[2] * b[0]) - (a[0] * b[2]), (a[0] * b[1]) - (a[1] * b[0])]
}