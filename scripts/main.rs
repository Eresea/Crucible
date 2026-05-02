pub struct PlayerController {
    pub speed: f32,
}

impl PlayerController {
    pub fn update(&mut self, delta_seconds: f32) {
        let _movement = self.speed * delta_seconds;
    }
}
