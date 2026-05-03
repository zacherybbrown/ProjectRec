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

    pub fn load_or_fallback<P: AsRef<Path>>(assets_path: P) -> anyhow::Result<Self> {
        match Self::load(&assets_path) {
            Ok(manager) => Ok(manager),
            Err(_) => Ok(Self { manifest: AssetManifest::fallback() }),
        }
    }
}

impl AssetManifest {
    pub fn fallback() -> Self {
        Self {
            base_avatar: AssetAvatar {
                name: "Fallback Avatar".to_string(),
                description: "A basic fallback player built from cubes and a rectangle.".to_string(),
                model: "fallback_body".to_string(),
            },
            costumes: vec![AssetItem {
                name: "Fallback Left Hand".to_string(),
                description: "A cube used as a fallback left hand.".to_string(),
                model: "fallback_cube".to_string(),
            }, AssetItem {
                name: "Fallback Right Hand".to_string(),
                description: "A cube used as a fallback right hand.".to_string(),
                model: "fallback_cube".to_string(),
            }],
            cosmetics: vec![AssetItem {
                name: "Fallback Body Rectangle".to_string(),
                description: "A rectangle used as a fallback body.".to_string(),
                model: "fallback_rect".to_string(),
            }],
        }
    }
}
