use egui::*;
use macroquad::prelude::*;
use std::fs;
use std::io::prelude::*;

pub struct MainMenu {}

impl MainMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self) -> Option<std::path::PathBuf> {
        let mut level = None;

        clear_background(BLACK);

        set_default_camera();

        egui_macroquad::ui(|ctx| {
            egui::SidePanel::left("side_panel", 200.0).show(ctx, |ui| {
                ui.heading("Shitty rhythm game");

                ui.label("Levels");

                ui.group(|ui| {
                    ScrollArea::auto_sized().show(ui, |ui| {
                        for entry in fs::read_dir("songs").unwrap() {
                            if let Ok(entry) = entry {
                                if entry.path().is_dir() {
                                    let response = ui.button(
                                        entry.path().file_name().unwrap().to_str().unwrap(),
                                    );

                                    if response.clicked() {
                                        level = Some(entry.path());
                                    }
                                }
                            }
                        }
                    });
                });
            });
        });

        egui_macroquad::draw();

        level
    }
}
