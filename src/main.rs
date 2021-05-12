mod sheet;

use macroquad::audio::*;
use macroquad::prelude::*;
use sheet::{ParseError, Sheet, Token, TokenStream};
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
    pub ichannel0: Texture2D,
    pub background: Material,
    pub sheet: Sheet,
}

impl Assets {
    pub async fn new() -> Self {
        let assets = Self {
            song: load_sound("assets/song.wav").await.unwrap(),
            death: load_sound("assets/death.wav").await.unwrap(),
            kick: load_sound("assets/kick.wav").await.unwrap(),
            shield: load_texture("assets/shield.png").await.unwrap(),
            heart: load_texture("assets/heart.png").await.unwrap(),
            projectile: load_texture("assets/projectile.png").await.unwrap(),
            noise: load_texture("assets/noise.png").await.unwrap(),
            ichannel0: load_texture("shader/iChannel0.png").await.unwrap(),
            background: load_material(
                VERTEX,
                &std::fs::read_to_string("shader/shader.glsl").unwrap(),
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
            sheet: Sheet::parse(&std::fs::read_to_string("sheet.sht").unwrap()).unwrap(),
        };

        assets.shield.set_filter(FilterMode::Nearest);
        assets.heart.set_filter(FilterMode::Nearest);
        assets.projectile.set_filter(FilterMode::Nearest);
        //assets.shield.set_filter(FilterMode::Nearest);

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
        (self.arrival_time - env.time) * env.speed * (bpm / 120.0) + 48.0
    }

    pub fn parse(tokens: &mut impl TokenStream, bpm: f32, start: f32) -> sheet::Result<Self> {
        let ty = tokens.next_token()?;

        if let Token::Projectile(ty) = ty {
            let direction = tokens.next_token()?;

            if let Token::Direction(direction) = direction {
                let time_offset = tokens.next_token()?;

                if let Token::TimeOffset(time_offset) = time_offset {
                    Ok(Self {
                        arrival_time: start + time_offset.time(bpm),
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
        let offset = vec2(angle.cos(), angle.sin()) * self.distance(env, assets.sheet.bpm);

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
        }
    }

    pub async fn update(&mut self, assets: &Assets) {
        if let Some(death) = &mut self.death {
            self.env.time += get_frame_time() * (1.0 - *death).max(0.0);
            *death += get_frame_time();
        } else {
            if is_key_pressed(KeyCode::W) {
                self.shield = Some(Direction::Up);
            }

            if is_key_pressed(KeyCode::S) {
                self.shield = Some(Direction::Down);
            }

            if is_key_pressed(KeyCode::A) {
                self.shield = Some(Direction::Left);
            }

            if is_key_pressed(KeyCode::D) {
                self.shield = Some(Direction::Right);
            }

            self.env.time += get_frame_time();

            let env = &self.env;
            let shield = &self.shield;
            let camera_shake = &mut self.camera_shake;
            let score = &mut self.score;
            let death = &mut self.death;

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

        if is_key_pressed(KeyCode::R) {
            *self = Self::new(assets).await;
            stop_sound(assets.song);
            play_sound_once(assets.song);
        }
    }

    pub fn draw(&mut self, assets: &Assets) {
        let offset = vec2(
            rand::gen_range(-self.camera_shake, self.camera_shake),
            rand::gen_range(-self.camera_shake, self.camera_shake),
        );

        let aspect = screen_width() / screen_height();

        set_camera(&Camera2D {
            offset,
            zoom: vec2(1.0 / (300.0 * aspect), -1.0 / 300.0),
            ..Default::default()
        });

        clear_background(BLACK);

        let resolution = vec2(screen_width(), screen_height());

        assets.background.set_texture("noise_texture", assets.noise);
        assets.background.set_texture("iChannel0", assets.ichannel0);
        assets.background.set_uniform("iTime", self.env.time);
        assets.background.set_uniform("iResolution", resolution);

        gl_use_material(assets.background);

        draw_rectangle(0.0, 0.0, 1.0, 1.0, WHITE);

        gl_use_default_material();

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

        draw_text(&format!("Score: {}", self.score), 15.0, 30.0, 50.0, WHITE);
    }
}

#[macroquad::main("SRG")]
async fn main() {
    let assets = Assets::new().await;
    let mut state = GameState::new(&assets).await;

    play_sound_once(assets.song);

    loop {
        state.update(&assets).await;
        state.draw(&assets);

        next_frame().await;
    }
}
