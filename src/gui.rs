use crate::assets::AssetManager;
use crate::client;
use crate::room::{load_registry, RoomInfo};
use crate::server::RoomServer;
use anyhow::Result;
use eframe::{egui, NativeOptions};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
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

#[derive(PartialEq)]
enum ToolMode {
    Select,
    Move,
    Scale,
    Rotate,
}

#[derive(Clone)]
struct SceneObject {
    id: usize,
    name: String,
    position: [f32; 3],
    rotation: [f32; 3],
    size: f32,
    color: egui::Color32,
}

#[derive(Serialize)]
struct SceneSaveObject {
    id: usize,
    name: String,
    position: [f32; 3],
    rotation: [f32; 3],
    size: f32,
    color: [u8; 3],
}

impl SceneObject {
    fn new(id: usize, name: impl Into<String>, position: [f32; 3], size: f32, color: egui::Color32) -> Self {
        Self {
            id,
            name: name.into(),
            position,
            rotation: [0.0, 0.0, 0.0],
            size,
            color,
        }
    }

    fn to_save(&self) -> SceneSaveObject {
        SceneSaveObject {
            id: self.id,
            name: self.name.clone(),
            position: self.position,
            rotation: self.rotation,
            size: self.size,
            color: [self.color.r(), self.color.g(), self.color.b()],
        }
    }
}

struct Avatar {
    position: [f32; 3],
    rotation_y: f32,
    color: egui::Color32,
}

impl Avatar {
    fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            rotation_y: 0.0,
            color: egui::Color32::from_rgb(220, 180, 130),
        }
    }
}

struct EditorState {
    selected: Option<usize>,
    tool: ToolMode,
    cursor: [i32; 3],
    size: f32,
    color: [u8; 3],
    action_message: String,
    snap: bool,
    grid_size: f32,
}

impl EditorState {
    fn new() -> Self {
        Self {
            selected: None,
            tool: ToolMode::Select,
            cursor: [0, 0, 5],
            size: 2.0,
            color: [160, 220, 160],
            action_message: "Editor ready.".to_string(),
            snap: true,
            grid_size: 1.0,
        }
    }
}

struct ProjectRecApp {
    mode: AppMode,
    game: GameState,
    editor: EditorState,
    world: Vec<SceneObject>,
    avatar: Avatar,
    cursor_locked: bool,
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
            world: vec![
                SceneObject::new(1, "Starter Cube", [0.0, 0.0, 8.0], 2.5, egui::Color32::from_rgb(120, 180, 255)),
                SceneObject::new(2, "Green Platform", [0.0, -1.0, 12.0], 8.0, egui::Color32::from_rgb(100, 180, 100)),
            ],
            avatar: Avatar::new([0.0, 0.0, 4.0]),
            cursor_locked: false,
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
                        self.cursor_locked = true;
                        self.status = "Game started. Press ESC to unlock cursor.".to_string();
                    }
                    if ui.button("Editor Mode").clicked() {
                        self.mode = AppMode::Editor;
                        self.cursor_locked = true;
                        self.status = "Editor started. Press ESC to unlock cursor.".to_string();
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
    fn world_to_camera(&self, point: [f32; 3]) -> [f32; 3] {
        let dx = point[0] - self.game.position[0];
        let dy = point[1] - self.game.position[1];
        let dz = point[2] - self.game.position[2];
        let yaw = self.game.yaw;
        let pitch = self.game.pitch;

        let cos_y = yaw.cos();
        let sin_y = yaw.sin();
        let xz = [dx * cos_y + dz * sin_y, -dx * sin_y + dz * cos_y];

        let cos_p = pitch.cos();
        let sin_p = pitch.sin();
        let x = xz[0];
        let y = dy * cos_p - xz[1] * sin_p;
        let z = dy * sin_p + xz[1] * cos_p;
        [x, y, z]
    }

    fn project_point(&self, point: [f32; 3], center: egui::Pos2, scale: f32) -> Option<(egui::Pos2, f32, [f32; 3])> {
        let mut cam = self.world_to_camera(point);
        if cam[2] <= 0.1 {
            cam[2] = 0.1;
        }
        let px = center.x + cam[0] / cam[2] * scale;
        let py = center.y - cam[1] / cam[2] * scale;
        Some((egui::pos2(px, py), cam[2], cam))
    }

    fn draw_cube(&self, painter: &egui::Painter, cube: &SceneObject, center: egui::Pos2, scale: f32, selected: bool) {
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

        let projected: Vec<(egui::Pos2, f32, [f32; 3])> = corners
            .iter()
            .filter_map(|point| self.project_point(*point, center, scale))
            .collect();
        if projected.len() != corners.len() {
            return;
        }

        let pts: Vec<egui::Pos2> = projected.iter().map(|entry| entry.0).collect();

        let faces = [
            [0, 1, 2, 3],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [1, 2, 6, 5],
            [0, 3, 7, 4],
        ];

        let mut face_data: Vec<(f32, Vec<egui::Pos2>)> = Vec::new();
        for indices in faces {
            let poly: Vec<egui::Pos2> = indices.iter().map(|&i| pts[i]).collect();
            let depth = indices.iter().map(|&i| projected[i].1).sum::<f32>() / 4.0;
            face_data.push((depth, poly));
        }

        face_data.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, polygon) in face_data {
            painter.add(egui::Shape::convex_polygon(
                polygon.clone(),
                cube.color,
                egui::Stroke::new(1.5, egui::Color32::BLACK),
            ));
        }

        let edge_indices = [
            (0, 1), (1, 2), (2, 3), (3, 0),
            (4, 5), (5, 6), (6, 7), (7, 4),
            (0, 4), (1, 5), (2, 6), (3, 7),
        ];

        for &(a, b) in &edge_indices {
            painter.line_segment([pts[a], pts[b]], (2.0, egui::Color32::BLACK));
        }

        if selected {
            for &(a, b) in &edge_indices {
                painter.line_segment([pts[a], pts[b]], (2.0, egui::Color32::WHITE));
            }
        }
    }

    fn draw_avatar(&self, painter: &egui::Painter, center: egui::Pos2, scale: f32) {
        let sin_y = self.avatar.rotation_y.to_radians().sin();
        let cos_y = self.avatar.rotation_y.to_radians().cos();
        let left_offset = [-0.9 * cos_y, 0.0, -0.9 * sin_y];
        let right_offset = [0.9 * cos_y, 0.0, 0.9 * sin_y];
        let head = SceneObject::new(0, "Head", [self.avatar.position[0], self.avatar.position[1] + 1.6, self.avatar.position[2]], 0.8, self.avatar.color);
        let body = SceneObject::new(0, "Body", [self.avatar.position[0], self.avatar.position[1] + 0.7, self.avatar.position[2]], 1.0, egui::Color32::from_rgb(100, 160, 220));
        let left_arm = SceneObject::new(0, "Left Arm", [self.avatar.position[0] + left_offset[0], self.avatar.position[1] + 0.7, self.avatar.position[2] + left_offset[2]], 0.4, egui::Color32::from_rgb(180, 180, 180));
        let right_arm = SceneObject::new(0, "Right Arm", [self.avatar.position[0] + right_offset[0], self.avatar.position[1] + 0.7, self.avatar.position[2] + right_offset[2]], 0.4, egui::Color32::from_rgb(180, 180, 180));
        let left_leg = SceneObject::new(0, "Left Leg", [self.avatar.position[0] - 0.35, self.avatar.position[1] - 0.8, self.avatar.position[2]], 0.5, egui::Color32::from_rgb(100, 100, 160));
        let right_leg = SceneObject::new(0, "Right Leg", [self.avatar.position[0] + 0.35, self.avatar.position[1] - 0.8, self.avatar.position[2]], 0.5, egui::Color32::from_rgb(100, 100, 160));
        let parts = [head, body, left_arm, right_arm, left_leg, right_leg];

        for part in parts {
            self.draw_cube(painter, &part, center, scale, false);
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, center: egui::Pos2, scale: f32) {
        let grid_color = egui::Color32::from_rgb(70, 90, 120);
        for i in -10..=10 {
            let start = [i as f32, 0.0, 2.0];
            let end = [i as f32, 0.0, 20.0];
            if let (Some((p1, _, _)), Some((p2, _, _))) = (self.project_point(start, center, scale), self.project_point(end, center, scale)) {
                painter.line_segment([p1, p2], (1.0, grid_color));
            }
            let start = [-10.0, 0.0, i as f32 + 2.0];
            let end = [10.0, 0.0, i as f32 + 2.0];
            if let (Some((p1, _, _)), Some((p2, _, _))) = (self.project_point(start, center, scale), self.project_point(end, center, scale)) {
                painter.line_segment([p1, p2], (1.0, grid_color));
            }
        }
    }

    fn draw_world(&self, painter: &egui::Painter, center: egui::Pos2, rect: egui::Rect, show_avatar: bool) {
        let ground_color = egui::Color32::from_rgb(40, 60, 80);
        let sky_color = egui::Color32::from_rgb(10, 25, 60);
        painter.rect_filled(rect, 0.0, sky_color);
        let horizon = egui::Rect::from_min_max(
            egui::pos2(rect.left(), center.y),
            egui::pos2(rect.right(), rect.bottom()),
        );
        painter.rect_filled(horizon, 0.0, ground_color);
        self.draw_grid(painter, center, 360.0);

        let mut ordered_objects = self.world.clone();
        ordered_objects.sort_by(|a, b| {
            let da = (a.position[0] - self.game.position[0]).powi(2)
                + (a.position[1] - self.game.position[1]).powi(2)
                + (a.position[2] - self.game.position[2]).powi(2);
            let db = (b.position[0] - self.game.position[0]).powi(2)
                + (b.position[1] - self.game.position[1]).powi(2)
                + (b.position[2] - self.game.position[2]).powi(2);
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });

        for object in &ordered_objects {
            let selected = self.editor.selected.map(|index| self.world[index].id == object.id).unwrap_or(false);
            self.draw_cube(painter, object, center, 360.0, selected);
        }

        if show_avatar {
            self.draw_avatar(painter, center, 360.0);
        }
    }

    fn update_game(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = 1.0 / 60.0;
        let (escape_pressed, w_pressed, s_pressed, a_pressed, d_pressed, jump_pressed, pointer_delta) = ctx.input(|input| {
            (
                input.key_pressed(egui::Key::Escape),
                input.key_down(egui::Key::W),
                input.key_down(egui::Key::S),
                input.key_down(egui::Key::A),
                input.key_down(egui::Key::D),
                input.key_pressed(egui::Key::Space),
                input.pointer.delta(),
            )
        });
        if escape_pressed {
            self.cursor_locked = !self.cursor_locked;
            if self.cursor_locked {
                self.status = "Cursor locked. Move mouse to look.".to_string();
            } else {
                self.status = "Cursor unlocked. Press ESC again to lock.".to_string();
            }
        }

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
            self.game.velocity_y = 4.0;
            self.game.grounded = false;
        }
        if self.cursor_locked {
            self.game.yaw += pointer_delta.x * 0.002;
            self.game.pitch = (self.game.pitch + pointer_delta.y * 0.002).clamp(-1.2, 1.2);
        }

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

        self.avatar.position = [self.game.position[0], self.game.position[1] - 1.6, self.game.position[2]];
        self.avatar.rotation_y = self.game.yaw.to_degrees();

        if self.cursor_locked {
            ctx.set_cursor_icon(egui::CursorIcon::None);
        } else {
            ctx.set_cursor_icon(egui::CursorIcon::Default);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Rec 3D World");
            ui.label("Move with WASD, jump with SPACE, mouse to look around.");
            ui.separator();
            let available = ui.max_rect();
            let painter = ui.painter();
            self.draw_world(painter, available.center(), available, false);
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Project Rec Editor");
            ui.label("Editor mode: scene, object, transform, and properties.");
            ui.separator();

            ui.horizontal(|ui| {
                ui.radio_value(&mut self.editor.tool, ToolMode::Select, "Select");
                ui.radio_value(&mut self.editor.tool, ToolMode::Move, "Move");
                ui.radio_value(&mut self.editor.tool, ToolMode::Scale, "Scale");
                ui.radio_value(&mut self.editor.tool, ToolMode::Rotate, "Rotate");
            });
            ui.checkbox(&mut self.editor.snap, "Snap to grid");
            ui.add(egui::Slider::new(&mut self.editor.grid_size, 0.5..=4.0).text("Grid size"));
            ui.separator();

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Scene Objects");
                    for (index, object) in self.world.iter().enumerate() {
                        if ui
                            .selectable_label(self.editor.selected == Some(index), &object.name)
                            .clicked()
                        {
                            self.editor.selected = Some(index);
                            self.editor.action_message = format!("Selected {}.", object.name);
                        }
                    }
                    if ui.button("Insert Cube").clicked() {
                        let id = self.world.len() + 1;
                        self.world.push(SceneObject::new(
                            id,
                            format!("Cube {}", id),
                            [self.editor.cursor[0] as f32, self.editor.cursor[1] as f32, self.editor.cursor[2] as f32],
                            self.editor.size,
                            egui::Color32::from_rgb(self.editor.color[0], self.editor.color[1], self.editor.color[2]),
                        ));
                        self.editor.selected = Some(self.world.len() - 1);
                        self.editor.action_message = "Inserted new cube.".to_string();
                    }
                    if ui.button("Duplicate Selected").clicked() {
                        if let Some(selected) = self.editor.selected {
                            if let Some(source) = self.world.get(selected).cloned() {
                                let id = self.world.len() + 1;
                                let mut clone = source.clone();
                                clone.id = id;
                                clone.name = format!("{} Copy", source.name);
                                clone.position[2] += 2.0;
                                self.world.push(clone);
                                self.editor.selected = Some(self.world.len() - 1);
                                self.editor.action_message = "Duplicated selected object.".to_string();
                            }
                        }
                    }
                    if ui.button("Delete Selected").clicked() {
                        if let Some(selected) = self.editor.selected {
                            if selected < self.world.len() {
                                self.world.remove(selected);
                                self.editor.selected = None;
                                self.editor.action_message = "Deleted selected object.".to_string();
                            }
                        }
                    }
                    ui.label(&self.editor.action_message);
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.heading("Object Properties");
                    if let Some(index) = self.editor.selected {
                        if let Some(object) = self.world.get_mut(index) {
                            ui.label(&object.name);
                            ui.horizontal(|ui| {
                                ui.label("Position");
                                ui.add(egui::DragValue::new(&mut object.position[0]).speed(0.5));
                                ui.add(egui::DragValue::new(&mut object.position[1]).speed(0.5));
                                ui.add(egui::DragValue::new(&mut object.position[2]).speed(0.5));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Size");
                                ui.add(egui::DragValue::new(&mut object.size).speed(0.1));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Color");
                                let mut r = object.color.r();
                                let mut g = object.color.g();
                                let mut b = object.color.b();
                                ui.add(egui::DragValue::new(&mut r).range(0..=255));
                                ui.add(egui::DragValue::new(&mut g).range(0..=255));
                                ui.add(egui::DragValue::new(&mut b).range(0..=255));
                                object.color = egui::Color32::from_rgb(r, g, b);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Rotation Y");
                                ui.add(egui::Slider::new(&mut object.rotation[1], -180.0..=180.0).text("deg"));
                            });
                            if ui.button("Focus Object").clicked() {
                                self.game.position = [object.position[0], object.position[1] + 1.5, object.position[2] - 8.0];
                                self.editor.action_message = format!("Focused camera on {}.", object.name);
                            }
                        }
                    } else {
                        ui.label("Select an object to edit its transform.");
                    }
                });
            });

            ui.separator();
            ui.label("Scene preview:");
            let preview_response = ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            let preview_rect = preview_response.rect;
            let preview_painter = ui.painter_at(preview_rect);
            self.draw_world(&preview_painter, preview_rect.center(), preview_rect, false);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save Level").clicked() {
                    let save_objects: Vec<SceneSaveObject> = self.world.iter().map(|obj| obj.to_save()).collect();
                    let json = serde_json::to_string_pretty(&save_objects);
                    match json {
                        Ok(contents) => {
                            let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                            path.push("saved_level.json");
                            match File::create(&path) {
                                Ok(mut file) => {
                                    if let Err(err) = file.write_all(contents.as_bytes()) {
                                        self.editor.action_message = format!("Save failed: {}", err);
                                    } else {
                                        self.editor.action_message = format!("Saved level to {}", path.display());
                                    }
                                }
                                Err(err) => {
                                    self.editor.action_message = format!("Save failed: {}", err);
                                }
                            }
                        }
                        Err(err) => {
                            self.editor.action_message = format!("Save failed: {}", err);
                        }
                    }
                }
                if ui.button("Back to Game").clicked() {
                    self.mode = AppMode::Game;
                    self.cursor_locked = true;
                    self.status = "Returned to game.".to_string();
                }
                if ui.button("Back to Menu").clicked() {
                    self.mode = AppMode::Menu;
                }
            });
        });
    }
}
