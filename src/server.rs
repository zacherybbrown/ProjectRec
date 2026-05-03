use crate::assets::AssetManager;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::room::{RoomInfo, RoomRegistry};
use crate::avatar::AvatarProfile;
use crate::transport::SkyTrain;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};

pub struct RoomServer {
    info: RoomInfo,
    asset_manager: AssetManager,
    clients: Arc<Mutex<HashMap<SocketAddr, AvatarProfile>>>,
    announcer: broadcast::Sender<ServerMessage>,
}

impl RoomServer {
    pub fn new(info: RoomInfo, asset_manager: AssetManager) -> Self {
        let (tx, _rx) = broadcast::channel(32);
        Self {
            info,
            asset_manager,
            clients: Arc::new(Mutex::new(HashMap::new())),
            announcer: tx,
        }
    }

    pub async fn run(self) -> Result<()> {
        self.write_registry()?;
        let address = format!("{}:{}", self.info.host, self.info.port);
        let listener = TcpListener::bind(&address)
            .await
            .with_context(|| format!("Failed to bind to {}", address))?;
        println!("Room server running on {}", address);
        println!("Room name: {}", self.info.name);
        println!("Loaded base avatar: {}", self.asset_manager.manifest.base_avatar.name);

        loop {
            let (socket, peer_addr) = listener.accept().await?;
            let room = self.info.clone();
            let asset_manager = self.asset_manager.clone();
            let clients = self.clients.clone();
            let announcer = self.announcer.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_connection(socket, peer_addr, room, asset_manager, clients, announcer).await {
                    eprintln!("Connection error from {}: {}", peer_addr, err);
                }
            });
        }
    }

    fn write_registry(&self) -> Result<()> {
        let registry = RoomRegistry {
            rooms: vec![self.info.clone()],
        };
        let json = serde_json::to_string_pretty(&registry)?;
        fs::write("rooms.json", json).context("Failed to write local room registry")?;
        Ok(())
    }
}

async fn handle_connection(
    socket: TcpStream,
    peer_addr: SocketAddr,
    room: RoomInfo,
    _asset_manager: AssetManager,
    clients: Arc<Mutex<HashMap<SocketAddr, AvatarProfile>>>,
    announcer: broadcast::Sender<ServerMessage>,
) -> Result<()> {
    let (reader, writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();
    let mut client_name = String::new();
    let mut _avatar = AvatarProfile::base("Guest");

    writer
        .write_all(format!("Welcome to room {}\n", room.name).as_bytes())
        .await?;
    writer.flush().await?;

    while reader.read_line(&mut line).await? != 0 {
        let trimmed = line.trim().to_string();
        line.clear();
        if trimmed.is_empty() {
            continue;
        }

        let msg: ClientMessage = match serde_json::from_str(&trimmed) {
            Ok(value) => value,
            Err(_) => {
                writer
                    .write_all(b"{\"Error\":\"Invalid command format\"}\n")
                    .await?;
                writer.flush().await?;
                continue;
            }
        };

        match msg {
            ClientMessage::JoinRequest { name, avatar: profile } => {
                client_name = name.clone();
                _avatar = profile;
                let mut guard = clients.lock().await;
                guard.insert(peer_addr, _avatar.clone());
                let response = ServerMessage::JoinAccepted {
                    welcome: format!("Joined room {} as {}", room.name, name),
                    current_room: room.clone(),
                };
                send_server_message(&mut writer, response).await?;
            }
            ClientMessage::ListRooms => {
                let rooms = load_registry().unwrap_or_default().rooms;
                let response = ServerMessage::RoomList(rooms);
                send_server_message(&mut writer, response).await?;
            }
            ClientMessage::InviteFriend { friend } => {
                let invite = ServerMessage::Invite {
                    from: client_name.clone(),
                    friend,
                };
                let _ = announcer.send(invite.clone());
                send_server_message(&mut writer, invite).await?;
            }
            ClientMessage::CallTrain { destination, train_type } => {
                let mut train = SkyTrain::new(room.name.clone(), destination.clone(), train_type.clone());
                train.board();
                let response = ServerMessage::TrainStatus { train: train.clone() };
                send_server_message(&mut writer, response).await?;
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let mut arrival = train.clone();
                arrival.arrive();
                let response = ServerMessage::TrainStatus { train: arrival };
                send_server_message(&mut writer, response).await?;
            }
            ClientMessage::Quit => {
                writer.write_all(b"{\"Info\":\"Goodbye\"}\n").await?;
                writer.flush().await?;
                break;
            }
        }
    }

    clients.lock().await.remove(&peer_addr);
    drop(announcer);
    Ok(())
}

async fn send_server_message(writer: &mut BufWriter<tokio::net::tcp::OwnedWriteHalf>, message: ServerMessage) -> Result<()> {
    let payload = serde_json::to_string(&message)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

fn load_registry() -> Option<RoomRegistry> {
    let contents = fs::read_to_string("rooms.json").ok()?;
    serde_json::from_str(&contents).ok()
}
