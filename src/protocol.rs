use crate::avatar::AvatarProfile;
use crate::room::RoomInfo;
use crate::transport::{SkyTrain, TrainType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    RoomList(Vec<RoomInfo>),
    JoinAccepted { welcome: String, current_room: RoomInfo },
    Invite { from: String, friend: String },
    TrainStatus { train: SkyTrain },
    Error { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    JoinRequest { name: String, avatar: AvatarProfile },
    ListRooms,
    CallTrain { destination: String, train_type: TrainType },
    InviteFriend { friend: String },
    Quit,
}

