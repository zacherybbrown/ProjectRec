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
    Editor,
}

struct GameState {
    position: [f32; 3],
    velocity_y: f32,
    yaw: f32,
    pitch: f32,
    grounded: bool,
}

impl GameState {
    fn new() -> Self {
        Self {
            position: [0.0, 1.5, -4.0],
            velocity_y: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            grounded: true,
        }
    }
}

#[derive(Clone)]
struct Cube {
    position: [f32; 3],
    size: f32,
    color: egui::Color32,
}

impl Cube {
    fn new(position: [f32; 3], size: f32, color: egui::Color32) -> Self {
        Self { position, size, color }
    }
}

struct EditorState {
    cursor: [i32; 3],
    size: f32,
    action_message: String,
}

impl EditorState {
    fn new() -> Self {
        Self {
            cursor: [0, 0, 5],
            size: 2.0,
            action_message: "Editor ready.".to_string(),
        }
    }
}

struct ProjectRecApp {
    mode: AppMode,
    game: GameState,
    editor: EditorState,
    world: Vec<Cube>,
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
            mode: AppMode::Menu,
            game: GameState::new(),
            editor: EditorState::new(),
            world: vec![Cube::new([0.0, 0.0, 8.0], 2.5, egui::Color32::from_rgb(120, 180, 255))],
            host_name: String::from("Lounge"),
            host_port: String::from("4000"),
            host_public: true,
            host_pc: true,
            host_pcvr: false,
            join_address: String::from("127.0.0.1:4000"),
            join_name: String::from("Player"),
            status: String::from("Ready."),
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
                    if ui.button("Editor Mode").clicked() {
                        self.mode = AppMode::Editor;
                        self.status = "Editor started. Place cubes into the world.".to_string();
                    }
                    ui.separator();
                    ui.label("Status:");
                    ui.label(&self.status);
                });
            }
            AppMode::Game => {
                self.update_game(ctx, frame);
            }
            AppMode::Editor => {
                self.update_editor(ctx, frame);
            }
        }
    }
}

impl ProjectRecApp {
    fn project_point(&self, point: [f32; 3], center: egui::Pos2, scale: f32) -> Option<(egui::Pos2, f32)> {
        let dx = point[0] - self.game.position[0];
        let dy = point[1] - self.game.position[1];
        let dz = point[2] - self.game.position[2];
        let yaw = self.game.yaw;
        let pitch = self.game.pitch;

        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let x = dx * cos_y + dz * sin_y;
        let z = -dx * sin_y + dz * cos_y;

        let cos_p = pitch.cos();
        let sin_p = pitch.sin();
        let y = dy * cos_p - z * sin_p;
        let z = dy * sin_p + z * cos_p;

        if z <= 0.1 {
            return None;
        }

        let px = center.x + x / z * scale;
        let py = center.y - y / z * scale;
        Some((egui::pos2(px, py), z))
    }

    fn draw_cube(&self, painter: &egui::Painter, cube: &Cube, center: egui::Pos2, scale: f32) {
        let half = cube.size * 0.5;
        let corners = [
            [cube.position[0] - half, cube.position[1] - half, cube.position[2] - half],
            [cube.position[0] + half, cube.position[1] - half, cube.position[2] - half],
            [cube.position[0] + half, cube.position[1] + half, cube.position[2] - half],
            [cube.position[0] - half, cube.position[1] + half, cube.position[2] - half],
            [cube.position[0] - half, cube.position[1] - half, cube.position[2] + half],
            [cube.position[0] + half, cube.position[1] - half, cube.position[2] + half],
            [cube.position[0] + half, cube.position[1] + half, cube.position[2] + half],
            [cube.position[0] - half, cube.position[1] + half, cube.position[2] + half],
        ];

        let projected: Vec<Option<(egui::Pos2, f32)>> = corners
            .iter()
            .map(|point| self.project_point(*point, center, scale))
            .collect();
        if projected.iter().any(|entry| entry.is_none()) {
            return;
        }

        let pts: Vec<egui::Pos2> = projected.iter().map(|entry| entry.unwrap().0).collect();
        let face_order = [
            ([0, 1, 2, 3], 0.85),
            ([1, 5, 6, 2], 0.75),
            ([3, 2, 6, 7], 0.95),
        ];

        for (indices, brightness) in face_order {
            let polygon: Vec<egui::Pos2> = indices.iter().map(|&i| pts[i]).collect();
            painter.add(egui::Shape::convex_polygon(
                polygon,
                egui::Color32::from_rgb(
                    (cube.color.r() as f32 * brightness) as u8,
                    (cube.color.g() as f32 * brightness) as u8,
                    (cube.color.b() as f32 * brightness) as u8,
                ),
                egui::Stroke::new(1.5, egui::Color32::BLACK),
            ));
        }
    }

    fn draw_world(&self, painter: &egui::Painter, center: egui::Pos2, rect: egui::Rect) {
        let ground_color = egui::Color32::from_rgb(40, 60, 80);
        let sky_color = egui::Color32::from_rgb(10, 25, 60);
        painter.rect_filled(rect, 0.0, sky_color);
        let horizon = egui::Rect::from_min_max(
            egui::pos2(rect.left(), center.y),
            egui::pos2(rect.right(), rect.bottom()),
        );
        painter.rect_filled(horizon, 0.0, ground_color);

        let mut ordered_cubes = self.world.clone();
        ordered_cubes.sort_by(|a, b| {
            let da = ((a.position[0] - self.game.position[0]).powi(2)
                + (a.position[1] - self.game.position[1]).powi(2)
                + (a.position[2] - self.game.position[2]).powi(2))
            .partial_cmp(&((b.position[0] - self.game.position[0]).powi(2)
                + (b.position[1] - self.game.position[1]).powi(2)
                + (b.position[2] - self.game.position[2]).powi(2)))
            .unwrap_or(std::cmp::Ordering::Equal);
            da
        });

        for cube in &ordered_cubes {
            self.draw_cube(painter, cube, center, 360.0);
        }
    }

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

        let forward = egui::vec2(self.game.yaw.sin(), self.game.yaw.cos());
        let right = egui::vec2(self.game.yaw.cos(), -self.game.yaw.sin());
        let speed = 120.0;
        let delta_move = (forward * movement.y + right * movement.x) * speed * dt;
        self.game.position[0] += delta_move.x;
        self.game.position[2] += delta_move.y;

        self.game.velocity_y -= 9.8 * dt;
        self.game.position[1] += self.game.velocity_y;
        if self.game.position[1] <= 0.0 {
            self.game.grounded = true;
            self.game.position[1] = 0.0;
            self.game.velocity_y = 0.0;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Rec 3D World");
            ui.label("Move with WASD, jump with SPACE, mouse to look around.");
            ui.separator();
            let available = ui.max_rect();
            let painter = ui.painter();
            self.draw_world(painter, available.center(), available);
            let center = available.center();
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
            ui.label(format!("Camera pos: {:.1}, {:.1}, {:.1}", self.game.position[0], self.game.position[1], self.game.position[2]));
            ui.label(format!("View yaw: {:.2}, pitch: {:.2}", self.game.yaw, self.game.pitch));
            ui.horizontal(|ui| {
                if ui.button("Editor Mode").clicked() {
                    self.mode = AppMode::Editor;
                    self.status = "Editor started.".to_string();
                }
                if ui.button("Return to Menu").clicked() {
                    self.mode = AppMode::Menu;
                }
            });
        });
    }

    fn update_editor(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let _available = egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Rec Editor");
            ui.label("Place and remove cubes in the 3D world.");
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.editor.cursor[0]).speed(1));
                ui.label("X");
                ui.add(egui::DragValue::new(&mut self.editor.cursor[1]).speed(1));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut self.editor.cursor[2]).speed(1));
                ui.label("Z");
            });
            ui.add(egui::Slider::new(&mut self.editor.size, 0.5..=4.0).text("Size"));
            if ui.button("Add Cube").clicked() {
                self.world.push(Cube::new(
                    [self.editor.cursor[0] as f32, self.editor.cursor[1] as f32, self.editor.cursor[2] as f32],
                    self.editor.size,
                    egui::Color32::from_rgb(160, 220, 160),
                ));
                self.editor.action_message = format!("Added cube at {} {} {}.", self.editor.cursor[0], self.editor.cursor[1], self.editor.cursor[2]);
            }
            if ui.button("Remove Nearest Cube").clicked() {
                if let Some(index) = self.world.iter().enumerate().min_by(|(_, a), (_, b)| {
                    let da = (a.position[0] - self.editor.cursor[0] as f32).abs()
                        + (a.position[1] - self.editor.cursor[1] as f32).abs()
                        + (a.position[2] - self.editor.cursor[2] as f32).abs();
                    let db = (b.position[0] - self.editor.cursor[0] as f32).abs()
                        + (b.position[1] - self.editor.cursor[1] as f32).abs()
                        + (b.position[2] - self.editor.cursor[2] as f32).abs();
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    self.world.remove(index.0);
                    self.editor.action_message = "Removed nearest cube.".to_string();
                }
            }
            ui.label(&self.editor.action_message);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Back to Game").clicked() {
                    self.mode = AppMode::Game;
                    self.status = "Returned to game.".to_string();
                }
                if ui.button("Back to Menu").clicked() {
                    self.mode = AppMode::Menu;
                }
            });
            ui.separator();
            ui.label("World cubes:");
            for cube in &self.world {
                ui.label(format!("Cube at ({:.0}, {:.0}, {:.0}) size {:.1}", cube.position[0], cube.position[1], cube.position[2], cube.size));
            }
            ui.add_space(8.0);
            ui.label("Scene preview:");
            let preview_response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            let preview_rect = preview_response.rect;
            let preview_painter = ui.painter_at(preview_rect);
            self.draw_world(&preview_painter, preview_rect.center(), preview_rect);
        });
    }
}
