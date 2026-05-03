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

struct ProjectRecApp {
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.rx.try_recv() {
            self.status = message;
        }

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
                            let assets = match AssetManager::load("assets") {
                                Ok(manifest) => manifest,
                                Err(err) => {
                                    tx.send(format!("Asset load failed: {}", err)).ok();
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
            ui.label("Status:");
            ui.label(&self.status);
        });
    }
}
