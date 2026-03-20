use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WitConfig {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub locations: Vec<LocationEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_units")]
    pub units: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            units: default_units(),
        }
    }
}

fn default_units() -> String {
    "imperial".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocationEntry {
    pub slug: String,
    pub name: String,
}

impl WitConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn has_location(&self, slug: &str) -> bool {
        self.locations.iter().any(|l| l.slug == slug)
    }

    pub fn add_location(&mut self, slug: String, name: String) {
        self.locations.push(LocationEntry { slug, name });
    }
}
