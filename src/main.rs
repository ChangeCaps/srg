mod game;
mod main_menu;
mod particles;
mod sheet;

use game::*;
use macroquad::prelude::*;
use main_menu::*;

#[macroquad::main("SRG")]
async fn main() {
    let mut main_menu = MainMenu::new();
    let mut game: Option<(Assets, GameState)> = None;

    loop {
        if let Some((assets, state)) = &mut game {
            state.update(assets).await;
            state.draw(assets);

            if is_key_pressed(KeyCode::Escape) {
                state.stop(assets);

                game = None;
            }
        } else {
            if let Some(level_path) = main_menu.update() {
                let assets = Assets::load(level_path).await;
                let mut state = GameState::new(&assets).await;

                state.start(&assets);

                game = Some((assets, state));
            }
        }

        next_frame().await;
    }
}
