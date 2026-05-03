use crate::avatar::AvatarProfile;
use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::TrainType;
use anyhow::{Context, Result};
use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

pub async fn run_client(address: &str, avatar_name: &str) -> Result<()> {
    let socket = TcpStream::connect(address)
        .await
        .with_context(|| format!("Failed to connect to room at {}", address))?;
    let (reader, writer) = socket.into_split();
    let reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);
    let mut line = String::new();

    let profile = AvatarProfile::base(avatar_name.to_string());
    let join = ClientMessage::JoinRequest {
        name: avatar_name.to_string(),
        avatar: profile,
    };
    let payload = serde_json::to_string(&join)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;

    tokio::spawn(async move {
        let mut remote_lines = String::new();
        let mut remote_reader = reader;
        while remote_reader.read_line(&mut remote_lines).await.unwrap_or(0) != 0 {
            if remote_lines.trim().is_empty() {
                remote_lines.clear();
                continue;
            }
            if let Ok(message) = serde_json::from_str::<ServerMessage>(remote_lines.trim()) {
                println!("[room] {:?}", message);
            } else {
                println!("[room] {}", remote_lines.trim());
            }
            remote_lines.clear();
        }
    });

    loop {
        print!("command> ");
        io::stdout().flush()?;
        line.clear();
        io::stdin().read_line(&mut line)?;
        let input = line.trim().to_lowercase();
        if input == "quit" || input == "exit" {
            let leave = ClientMessage::Quit;
            let payload = serde_json::to_string(&leave)?;
            writer.write_all(payload.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
            break;
        }

        let command = parse_command(&input)?;
        let payload = serde_json::to_string(&command)?;
        writer.write_all(payload.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}

fn parse_command(input: &str) -> Result<ClientMessage> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    match parts.as_slice() {
        ["list"] => Ok(ClientMessage::ListRooms),
        ["invite", friend] => Ok(ClientMessage::InviteFriend {
            friend: friend.to_string(),
        }),
        ["train", destination, "public"] => Ok(ClientMessage::CallTrain {
            destination: destination.to_string(),
            train_type: TrainType::Public,
        }),
        ["train", destination, "private"] => Ok(ClientMessage::CallTrain {
            destination: destination.to_string(),
            train_type: TrainType::Private,
        }),
        _ => Err(anyhow::anyhow!("Unknown command. Use list, invite <friend>, train <destination> public|private, quit")),
    }
}
