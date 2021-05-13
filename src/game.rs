use crate::particles::*;
use crate::sheet::{ParseError, Sheet, Token, TokenStream};
use macroquad::audio::*;
use macroquad::prelude::*;
use std::f32::consts::PI;

const VERTEX: &str = r#"
#version 450

layout(location = 0) in vec3 position;

layout(location = 0) out vec2 uv;

void main() {
    vec2 pos = position.xy * 2.0 - 1.0;
    gl_Position = vec4(pos, 0.0, 1.0);
    uv = pos;
}
"#;

pub struct Assets {
    pub song: Sound,
    pub death: Sound,
    pub kick: Sound,
    pub shield: Texture2D,
    pub heart: Texture2D,
    pub projectile: Texture2D,
    pub noise: Texture2D,
    pub ichannel0: Option<Texture2D>,
    pub particle: Texture2D,
    pub background: Material,
    pub sheet: Sheet,
}

impl Assets {
    pub async fn load(song_path: std::path::PathBuf) -> Self {
        let ichannel0 = song_path.join("shader/iChannel0.png");

        let ichannel0 = if ichannel0.exists() {
            Some(load_texture(ichannel0.to_str().unwrap())
                .await
                .unwrap())
        } else {
            None
        };

        let assets = Self {
            song: load_sound(song_path.join("song.wav").to_str().unwrap())
                .await
                .unwrap(),
            death: load_sound("assets/death.wav").await.unwrap(),
            kick: load_sound("assets/kick.wav").await.unwrap(),
            shield: load_texture("assets/shield.png").await.unwrap(),
            heart: load_texture("assets/heart.png").await.unwrap(),
            projectile: load_texture("assets/projectile.png").await.unwrap(),
            noise: load_texture("assets/noise.png").await.unwrap(),
            ichannel0,
            particle: load_texture("assets/particle.png").await.unwrap(),
            background: load_material(
                VERTEX,
                &std::fs::read_to_string(song_path.join("shader/shader.glsl").to_str().unwrap())
                    .unwrap(),
                MaterialParams {
                    textures: vec!["noise_texture".to_string(), "iChannel0".to_string()],
                    uniforms: vec![
                        ("iTime".to_string(), UniformType::Float1),
                        ("iResolution".to_string(), UniformType::Float2),
                    ],
                    ..Default::default()
                },
            )
            .unwrap(),
            sheet: Sheet::parse(&std::fs::read_to_string(song_path.join("sheet.sht")).unwrap())
                .unwrap(),
        };

        assets.shield.set_filter(FilterMode::Nearest);
        assets.heart.set_filter(FilterMode::Nearest);
        assets.projectile.set_filter(FilterMode::Nearest);

        assets
    }
}

pub struct Env {
    pub time: f32,
    pub speed: f32,
}

impl Env {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            speed: 128.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ProjectileType {
    Normal,
}

pub enum ProjectileHit {
    None,
    Blocked,
    Hit,
}

#[derive(Clone)]
pub struct Projectile {
    pub arrival_time: f32,
    pub direction: Direction,
    pub ty: ProjectileType,
}

impl Projectile {
    pub fn random(time: f32) -> Self {
        Self {
            arrival_time: time,
            direction: Direction::random(),
            ty: ProjectileType::Normal,
        }
    }

    pub fn distance(&self, env: &Env, bpm: f32) -> f32 {
        (self.arrival_time - env.time) * env.speed * (bpm / 60.0) + 48.0
    }

    pub fn position(&self, env: &Env, bpm: f32) -> Vec2 {
        let angle = self.direction.angle();

        vec2(angle.cos(), angle.sin()) * self.distance(env, bpm)
    }

    pub fn parse(
        tokens: &mut impl TokenStream,
        bpm: f32,
        offset: f32,
    ) -> crate::sheet::Result<Self> {
        let ty = tokens.next_token()?;

        if let Token::Projectile(ty) = ty {
            let direction = tokens.next_token()?;

            if let Token::Direction(direction) = direction {
                let time_offset = tokens.next_token()?;

                if let Token::TimeOffset(time_offset) = time_offset {
                    Ok(Self {
                        arrival_time: offset + time_offset.time(bpm),
                        direction,
                        ty,
                    })
                } else {
                    Err(ParseError::UnexpectedToken(time_offset))
                }
            } else {
                Err(ParseError::UnexpectedToken(direction))
            }
        } else {
            Err(ParseError::UnexpectedToken(ty))
        }
    }

    pub fn update(&self, env: &Env, shield: &Option<Direction>, bpm: f32) -> ProjectileHit {
        let blocking = if let Some(shield) = shield {
            *shield == self.direction
        } else {
            false
        };

        let distance = self.distance(env, bpm);

        if blocking && distance < 48.0 {
            ProjectileHit::Blocked
        } else if distance <= 16.0 {
            ProjectileHit::Hit
        } else {
            ProjectileHit::None
        }
    }

    pub fn draw(&self, env: &Env, assets: &Assets) {
        let angle = self.direction.angle();
        let offset = self.position(env, assets.sheet.bpm);

        draw_texture_ex(
            assets.projectile,
            offset.x - assets.projectile.width() / 2.0,
            offset.y - assets.projectile.height() / 2.0,
            WHITE,
            DrawTextureParams {
                rotation: angle,
                ..Default::default()
            },
        );
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn random() -> Self {
        match rand::gen_range(0u8, 4) {
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            _ => unreachable!(),
        }
    }

    pub fn angle(&self) -> f32 {
        match self {
            Self::Right => 0.0,
            Self::Up => -PI / 2.0,
            Self::Left => PI,
            Self::Down => PI / 2.0,
        }
    }
}

pub struct GameState {
    pub shield: Option<Direction>,
    pub env: Env,
    pub projectiles: Vec<Projectile>,
    pub camera_shake: f32,
    pub score: u32,
    pub death: Option<f32>,
    pub particles: ParticleSystem,
}

impl GameState {
    pub async fn new(assets: &Assets) -> Self {
        Self {
            shield: None,
            env: Env::new(),
            projectiles: assets.sheet.projectiles.clone(),
            camera_shake: 0.0,
            score: 0,
            death: None,
            particles: ParticleSystem::new(),
        }
    }

    pub fn start(&mut self, assets: &Assets) {
        play_sound_once(assets.song);
    }

    pub fn stop(&mut self, assets: &Assets) {
        stop_sound(assets.song);
    }

    pub async fn restart(&mut self, assets: &Assets) {
        *self = Self::new(assets).await;
        self.start(assets);
    }

    pub async fn update(&mut self, assets: &Assets) {
        let death_frame_time = get_frame_time() * (1.0 - self.death.unwrap_or(0.0)).max(0.0);

        self.env.time += death_frame_time;

        if let Some(death) = &mut self.death {
            *death += get_frame_time();
        } else {
            if is_key_pressed(KeyCode::W) || is_key_pressed(KeyCode::Up) {
                self.shield = Some(Direction::Up);
            }

            if is_key_pressed(KeyCode::S) || is_key_pressed(KeyCode::Down) {
                self.shield = Some(Direction::Down);
            }

            if is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::Left) {
                self.shield = Some(Direction::Left);
            }

            if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Right) {
                self.shield = Some(Direction::Right);
            }

            let env = &self.env;
            let shield = &self.shield;
            let camera_shake = &mut self.camera_shake;
            let score = &mut self.score;
            let death = &mut self.death;
            let particles = &mut self.particles;

            self.projectiles.retain(|projectile| {
                let hit = projectile.update(env, shield, assets.sheet.bpm);

                let retain = match hit {
                    ProjectileHit::None => true,
                    ProjectileHit::Blocked => false,
                    ProjectileHit::Hit => true,
                };

                if !retain {
                    *camera_shake += 0.01;
                    *score += 1;
                    play_sound_once(assets.kick);

                    let angle = projectile.direction.angle();

                    let explosion = DirectionalExplosion {
                        texture: Some(assets.particle),
                        amount: 10,
                        position: projectile.position(env, assets.sheet.bpm),
                        direction: angle - 0.2..angle + 0.2,
                        speed: 128.0..338.0,
                        size: 10.0,
                        life_time: 5.0,
                        color: WHITE,
                        rotation: 0.0..std::f32::consts::TAU,
                        angular_velocity: -std::f32::consts::PI..std::f32::consts::PI,
                        ..Default::default()
                    };

                    particles.spawn(&explosion);
                }

                if let ProjectileHit::Hit = hit {
                    *death = Some(0.0);
                    stop_sound(assets.song);

                    *camera_shake = 0.0;

                    play_sound_once(assets.death);
                }

                retain
            });

            self.camera_shake *= 0.9;
    
            // env
            self.env.speed += get_frame_time() * 2.0;
        }

        self.particles.update(death_frame_time);

        if is_key_pressed(KeyCode::R) {
            self.restart(assets).await;
        }
    }

    pub fn draw(&mut self, assets: &Assets) {
        let offset = vec2(
            rand::gen_range(-self.camera_shake, self.camera_shake),
            rand::gen_range(-self.camera_shake, self.camera_shake),
        );

        //let aspect = screen_width() / screen_height();

        set_camera(&Camera2D {
            offset,
            zoom: vec2(
                1.0 / (screen_width() / 2.0).floor(),
                -1.0 / (screen_height() / 2.0).floor(),
            ),
            ..Default::default()
        });

        clear_background(BLACK);

        let resolution = vec2(screen_width(), screen_height());

        assets.background.set_texture("noise_texture", assets.noise);

        if let Some(ichannel0) = assets.ichannel0 {
            assets.background.set_texture("iChannel0", ichannel0);
        }
        
        assets.background.set_uniform("iTime", self.env.time);
        assets.background.set_uniform("iResolution", resolution);

        gl_use_material(assets.background);

        draw_rectangle(0.0, 0.0, 1.0, 1.0, WHITE);

        gl_use_default_material();

        self.particles.draw();

        // projectiles
        for projectile in &self.projectiles {
            projectile.draw(&self.env, assets);
        }

        // heart
        draw_texture(
            assets.heart,
            -assets.heart.width() / 2.0,
            -assets.heart.height() / 2.0,
            WHITE,
        );

        // shield
        if let Some(shield) = &self.shield {
            let angle = shield.angle();
            let offset = vec2(angle.cos(), angle.sin()) * 32.0;

            draw_texture_ex(
                assets.shield,
                offset.x - assets.shield.width() / 2.0,
                offset.y - assets.shield.height() / 2.0,
                WHITE,
                DrawTextureParams {
                    rotation: angle,
                    ..Default::default()
                },
            );
        }

        set_default_camera();

        let bps = assets.sheet.bpm / 60.0;
        let beat = (self.env.time * bps * 4.0).floor() as u32;

        draw_text(&format!("Score: {}", self.score), 15.0, 30.0, 50.0, WHITE);
        draw_text(
            &format!("{};{}|{}", beat % 4, (beat / 4) % 4, beat / 16),
            500.0,
            30.0,
            50.0,
            WHITE,
        );
    }
}
