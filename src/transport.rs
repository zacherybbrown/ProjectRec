use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrainType {
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyTrain {
    pub origin: String,
    pub destination: String,
    pub train_type: TrainType,
    pub in_transit: bool,
}

impl SkyTrain {
    pub fn new(origin: impl Into<String>, destination: impl Into<String>, train_type: TrainType) -> Self {
        Self {
            origin: origin.into(),
            destination: destination.into(),
            train_type,
            in_transit: false,
        }
    }

    pub fn board(&mut self) {
        self.in_transit = true;
    }

    pub fn arrive(&mut self) {
        self.in_transit = false;
    }
}
