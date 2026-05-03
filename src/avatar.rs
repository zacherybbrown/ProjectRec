use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarProfile {
    pub name: String,
    pub costume: Option<String>,
    pub cosmetics: Vec<String>,
}

impl AvatarProfile {
    pub fn base(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            costume: None,
            cosmetics: Vec::new(),
        }
    }
}
