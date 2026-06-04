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

    fn cam_from(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        [
            self.cam_to[0] + self.radius * cp * self.yaw.sin(),
            self.cam_to[1] + self.radius * self.pitch.sin(),
            self.cam_to[2] + self.radius * cp * self.yaw.cos(),
        ]
    }

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
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

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

    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        const MAX_PITCH: f32 = 1.4;
        self.yaw   += yaw;
        self.pitch  = (self.pitch + pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }

    pub fn dolly(&mut self, amount: f32) {
        self.radius = (self.radius - amount).max(0.1);
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot_product(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let magnitude = dot_product(a, a).sqrt();
    a.map(|x| x / magnitude)
}

fn cross_product(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
