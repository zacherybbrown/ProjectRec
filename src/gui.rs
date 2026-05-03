use crate::assets::AssetManager;
use crate::client;
use crate::room::{load_registry, RoomInfo};
use crate::server::RoomServer;
use anyhow::Result;
use eframe::{egui, NativeOptions};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub fn run_gui() -> Result<()> {
    let options = NativeOptions::default();
    let result = eframe::run_native(
        "Project Rec GUI",
        options,
        Box::new(|_cc| Ok(Box::new(ProjectRecApp::new()))),
    );
    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow::anyhow!("GUI failed: {}", err)),
    }
}

enum AppMode {
    Menu,
    Game,
}

struct GameState {
    position: egui::Pos2,
    velocity_y: f32,
    yaw: f32,
    pitch: f32,
    grounded: bool,
}

impl GameState {
    fn new() -> Self {
        Self {
            position: egui::pos2(0.0, 0.0),
            velocity_y: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            grounded: true,
        }
    }
}

struct ProjectRecApp {
    mode: AppMode,
    game: GameState,
    host_name: String,
    host_port: String,
    host_public: bool,
    host_pc: bool,
    host_pcvr: bool,
    join_address: String,
    join_name: String,
    status: String,
    room_list: Vec<String>,
    tx: Sender<String>,
    rx: Receiver<String>,
}

impl ProjectRecApp {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            host_name: String::from("Lounge"),
            host_port: String::from("4000"),
            host_public: true,
            host_pc: true,
            host_pcvr: false,
            join_address: String::from("127.0.0.1:4000"),
            join_name: String::from("Player"),
            status: String::from("Ready."),
            mode: AppMode::Menu,
            game: GameState::new(),
            room_list: Vec::new(),
            tx,
            rx,
        };
        app.refresh_rooms();
        app
    }

    fn refresh_rooms(&mut self) {
        self.room_list.clear();
        if let Some(registry) = load_registry() {
            for room in registry.rooms {
                self.room_list.push(format!("{} at {}  public={}", room.name, room.address(), room.public));
            }
            if self.room_list.is_empty() {
                self.room_list.push("No connected rooms found.".to_string());
            }
        } else {
            self.room_list.push("No local room registry found.".to_string());
        }
    }
}

impl eframe::App for ProjectRecApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            self.status = message;
        }

        match self.mode {
            AppMode::Menu => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Project Rec GUI");
                    ui.horizontal(|ui| {
                        ui.label("PC mode");
                        ui.checkbox(&mut self.host_pc, "");
                        ui.label("PCVR mode");
                        ui.checkbox(&mut self.host_pcvr, "");
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Host a room");
                            ui.text_edit_singleline(&mut self.host_name);
                            ui.text_edit_singleline(&mut self.host_port);
                            ui.checkbox(&mut self.host_public, "Public room");
                            if ui.button("Start Hosting").clicked() {
                                let name = self.host_name.clone();
                                let port = self.host_port.clone();
                                let public = self.host_public;
                                let pc = self.host_pc;
                                let pcvr = self.host_pcvr;
                                let tx = self.tx.clone();
                                self.status = "Starting host...".to_string();
                                thread::spawn(move || {
                                    if !pc && !pcvr {
                                        tx.send("Host requires PC or PCVR mode.".to_string()).ok();
                                        return;
                                    }
                                    let port = port.parse::<u16>().unwrap_or(4000);
                                    let room_info = RoomInfo {
                                        id: format!("room-{}", port),
                                        name,
                                        host: "127.0.0.1".to_string(),
                                        port,
                                        public,
                                    };
                                    let assets = match AssetManager::load_or_fallback("assets") {
                                        Ok(manifest) => manifest,
                                        Err(err) => {
                                            tx.send(format!("Asset fallback failed: {}", err)).ok();
                                            return;
                                        }
                                    };
                                    let server = RoomServer::new(room_info, assets);
                                    let runtime = tokio::runtime::Runtime::new();
                                    if let Err(err) = runtime {
                                        tx.send(format!("Failed to start runtime: {}", err)).ok();
                                        return;
                                    }
                                    let runtime = runtime.unwrap();
                                    if let Err(err) = runtime.block_on(server.run()) {
                                        tx.send(format!("Host server error: {}", err)).ok();
                                    }
                                });
                            }
                        });

                        ui.separator();

                        ui.vertical(|ui| {
                            ui.label("Join a room");
                            ui.text_edit_singleline(&mut self.join_address);
                            ui.text_edit_singleline(&mut self.join_name);
                            if ui.button("Join Room").clicked() {
                                let address = self.join_address.clone();
                                let name = self.join_name.clone();
                                let tx = self.tx.clone();
                                self.status = "Connecting to room...".to_string();
                                thread::spawn(move || {
                                    let runtime = tokio::runtime::Runtime::new();
                                    if let Err(err) = runtime {
                                        tx.send(format!("Runtime failed: {}", err)).ok();
                                        return;
                                    }
                                    let runtime = runtime.unwrap();
                                    let result = runtime.block_on(client::run_gui_client(&address, &name));
                                    let message = match result {
                                        Ok(response) => format!("Join success: {}", response.trim()),
                                        Err(err) => format!("Join failed: {}", err),
                                    };
                                    tx.send(message).ok();
                                });
                            }
                        });
                    });

                    ui.separator();
                    if ui.button("Refresh room list").clicked() {
                        self.refresh_rooms();
                    }
                    ui.label("Connected rooms:");
                    for room_text in &self.room_list {
                        ui.label(room_text);
                    }
                    ui.separator();
                    if ui.button("Start Game").clicked() {
                        self.mode = AppMode::Game;
                        self.status = "Game started. WASD move, SPACE jump, mouse look.".to_string();
                    }
                    ui.separator();
                    ui.label("Status:");
                    ui.label(&self.status);
                });
            }
            AppMode::Game => {
                self.update_game(ctx, frame);
            }
        }
    }
}

impl ProjectRecApp {
    fn update_game(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = 1.0 / 60.0;
        let (w_pressed, s_pressed, a_pressed, d_pressed, jump_pressed, pointer_delta) = ctx.input(|input| {
            (
                input.key_down(egui::Key::W),
                input.key_down(egui::Key::S),
                input.key_down(egui::Key::A),
                input.key_down(egui::Key::D),
                input.key_pressed(egui::Key::Space),
                input.pointer.delta(),
            )
        });
        let mut movement = egui::vec2(0.0, 0.0);

        if w_pressed {
            movement.y -= 1.0;
        }
        if s_pressed {
            movement.y += 1.0;
        }
        if a_pressed {
            movement.x -= 1.0;
        }
        if d_pressed {
            movement.x += 1.0;
        }
        if jump_pressed && self.game.grounded {
            self.game.velocity_y = 6.0;
            self.game.grounded = false;
        }
        self.game.yaw += pointer_delta.x * 0.002;
        self.game.pitch = (self.game.pitch + pointer_delta.y * 0.002).clamp(-1.2, 1.2);

        let forward = egui::vec2(self.game.yaw.cos(), self.game.yaw.sin());
        let right = egui::vec2(-forward.y, forward.x);
        let speed = 120.0;
        let delta_move = (forward * movement.y + right * movement.x) * speed * dt;
        self.game.position += delta_move;

        self.game.velocity_y -= 9.8 * dt;
        self.game.position.y += self.game.velocity_y;
        if self.game.position.y <= 0.0 {
            self.game.grounded = true;
            self.game.position.y = 0.0;
            self.game.velocity_y = 0.0;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Rec Game");
            ui.label("Default fallback player: 2 cubes for hands and 1 rectangle for body.");
            ui.label("Use WASD to move, SPACE to jump, mouse to look.");
            ui.separator();
            let available = ui.max_rect();
            let painter = ui.painter();
            let center = available.center();
            let body = egui::Rect::from_center_size(center, egui::vec2(64.0, 120.0));
            painter.rect_filled(body, 8.0, egui::Color32::from_rgb(100, 180, 240));
            let left_hand = egui::Rect::from_center_size(center + egui::vec2(-40.0, 20.0), egui::vec2(24.0, 24.0));
            let right_hand = egui::Rect::from_center_size(center + egui::vec2(40.0, 20.0), egui::vec2(24.0, 24.0));
            painter.rect_filled(left_hand, 8.0, egui::Color32::from_rgb(180, 120, 100));
            painter.rect_filled(right_hand, 8.0, egui::Color32::from_rgb(180, 120, 100));
            let crosshair_size = 12.0;
            painter.line_segment(
                [center - egui::vec2(crosshair_size, 0.0), center + egui::vec2(crosshair_size, 0.0)],
                (2.0, egui::Color32::WHITE),
            );
            painter.line_segment(
                [center - egui::vec2(0.0, crosshair_size), center + egui::vec2(0.0, crosshair_size)],
                (2.0, egui::Color32::WHITE),
            );
            ui.separator();
            ui.label(format!("Position: {:.1}, {:.1}", self.game.position.x, self.game.position.y));
            ui.label(format!("View yaw: {:.2}, pitch: {:.2}", self.game.yaw, self.game.pitch));
            if ui.button("Return to Menu").clicked() {
                self.mode = AppMode::Menu;
            }
        });
    }
}
