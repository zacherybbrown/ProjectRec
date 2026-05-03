#![allow(dead_code)]

use anyhow::Context;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct AssetManifest {
    pub base_avatar: AssetAvatar,
    pub costumes: Vec<AssetItem>,
    pub cosmetics: Vec<AssetItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssetAvatar {
    pub name: String,
    pub description: String,
    pub model: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssetItem {
    pub name: String,
    pub description: String,
    pub model: String,
}

#[derive(Clone)]
pub struct AssetManager {
    pub manifest: AssetManifest,
}

impl AssetManager {
    pub fn load<P: AsRef<Path>>(assets_path: P) -> anyhow::Result<Self> {
        let path = assets_path.as_ref().join("manifest.json");
        let json = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read asset manifest {}", path.display()))?;
        let manifest: AssetManifest = serde_json::from_str(&json)
            .with_context(|| format!("Failed to parse asset manifest {}", path.display()))?;
        Ok(Self { manifest })
    }
}
