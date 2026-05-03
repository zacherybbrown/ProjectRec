mod assets;
mod avatar;
mod client;
mod protocol;
mod room;
mod server;
mod transport;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use crate::assets::AssetManager;
use crate::room::RoomInfo;
use crate::server::RoomServer;

#[derive(Parser)]
#[command(author, version, about = "Project Rec is a social room transport experience in Rust.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Host {
        #[arg(long)]
        room_name: String,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        public: bool,
        #[arg(long)]
        pcvr: bool,
    },
    Join {
        #[arg(long)]
        address: String,
        #[arg(long)]
        name: String,
    },
    List {},
    Info {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Host { room_name, port, public, pcvr } => {
            if !pcvr {
                anyhow::bail!("Room creation is allowed only when PCVR mode is enabled.");
            }
            let port = port.unwrap_or(4000);
            let room_id = format!(
                "room-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis()
            );
            let room_info = RoomInfo {
                id: room_id,
                name: room_name,
                host: "127.0.0.1".to_string(),
                port,
                public,
            };
            let assets = AssetManager::load("assets").context("Failed to load assets")?;
            let server = RoomServer::new(room_info, assets);
            server.run().await?;
        }
        Commands::Join { address, name } => {
            crate::client::run_client(&address, &name).await?;
        }
        Commands::List {} => {
            let registry = load_registry().unwrap_or_default();
            println!("Available rooms:");
            for room in registry.rooms {
                println!("- {} at {}  public={}", room.name, room.address(), room.public);
            }
        }
        Commands::Info {} => {
            println!("Project Rec is a social experience built with Rust.");
            println!("Host rooms with --pcvr enabled and join via room address.");
            println!("Use public or private sky trains to move between rooms.");
        }
    }
    Ok(())
}

fn load_registry() -> Option<crate::room::RoomRegistry> {
    let contents = std::fs::read_to_string("rooms.json").ok()?;
    serde_json::from_str(&contents).ok()
}
