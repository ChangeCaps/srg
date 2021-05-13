use macroquad::prelude::*;

pub trait ParticleSpawner {
    fn spawn_particles(&self) -> Vec<Particle>;
}

#[derive(Default)]
pub struct DirectionalExplosion {
    pub texture: Option<Texture2D>,
    pub amount: usize,
    pub color: Color,
    pub position: Vec2,
    pub speed: std::ops::Range<f32>,
    pub direction: std::ops::Range<f32>,
    pub rotation: std::ops::Range<f32>,
    pub angular_velocity: std::ops::Range<f32>,
    pub life_time: f32,
    pub size: f32,
}

impl ParticleSpawner for DirectionalExplosion {
    fn spawn_particles(&self) -> Vec<Particle> {
        (0..self.amount)
            .into_iter()
            .map(|_| {
                let direction = rand::gen_range(self.direction.start, self.direction.end);
                let speed = rand::gen_range(self.speed.start, self.speed.end);
                let velocity = vec2(direction.cos(), direction.sin()) * speed;
                let angular_velocity =
                    rand::gen_range(self.angular_velocity.start, self.angular_velocity.end);

                Particle {
                    texture: self.texture.clone(),
                    position: self.position,
                    rotation: rand::gen_range(self.rotation.start, self.rotation.end),
                    velocity,
                    angular_velocity,
                    size: self.size,
                    color: self.color,
                    life: 0.0,
                    life_time: self.life_time,
                }
            })
            .collect()
    }
}

pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub rotation: f32,
    pub angular_velocity: f32,
    pub texture: Option<Texture2D>,
    pub color: Color,
    pub size: f32,
    pub life: f32,
    pub life_time: f32,
}

impl Particle {
    pub fn update(&mut self, frame_time: f32) {
        self.position += self.velocity * frame_time;
        self.rotation += self.angular_velocity * frame_time;
        self.life += frame_time;

        self.color.a = 1.0 - self.life / self.life_time;
    }

    pub fn is_alive(&self) -> bool {
        self.life < self.life_time
    }

    pub fn draw(&self) {
        if let Some(texture) = self.texture {
            draw_texture_ex(
                texture,
                self.position.x - texture.width() / 2.0,
                self.position.y - texture.height() / 2.0,
                self.color,
                DrawTextureParams {
                    rotation: self.rotation,
                    ..Default::default()
                },
            )
        } else {
            draw_circle(self.position.x, self.position.y, self.size, self.color);
        }
    }
}

pub struct ParticleSystem {
    pub particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self { particles: vec![] }
    }

    pub fn spawn(&mut self, spawner: &impl ParticleSpawner) {
        let mut particles = spawner.spawn_particles();

        self.particles.append(&mut particles);
    }

    pub fn update(&mut self, frame_time: f32) {
        for particle in &mut self.particles {
            particle.update(frame_time);
        }

        self.particles.retain(|p| p.is_alive());
    }

    pub fn draw(&self) {
        for particle in &self.particles {
            particle.draw();
        }
    }
}
