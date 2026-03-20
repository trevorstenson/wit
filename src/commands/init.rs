use std::path::PathBuf;

use colored::Colorize;

use crate::config::WitConfig;
use crate::error::Result;
use crate::git::repo::WitRepo;

pub fn default_wit_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not find home directory")
        .join(".wit")
}

pub fn run(path: Option<String>) -> Result<()> {
    let wit_path = path.map(PathBuf::from).unwrap_or_else(default_wit_path);

    if wit_path.join(".git").exists() {
        println!(
            "  {} wit repo already exists at {}",
            "!".yellow(),
            wit_path.display()
        );
        return Ok(());
    }

    std::fs::create_dir_all(&wit_path).map_err(|e| {
        anyhow::anyhow!("failed to create directory {}: {}", wit_path.display(), e)
    })?;

    // Create locations directory
    std::fs::create_dir_all(wit_path.join("locations")).map_err(|e| {
        anyhow::anyhow!("failed to create locations dir: {}", e)
    })?;

    // Write default config
    let config = WitConfig::default();
    config.save(&wit_path.join("wit.toml")).map_err(|e| {
        anyhow::anyhow!("failed to write config: {}", e)
    })?;

    // Init git repo
    WitRepo::init(&wit_path)?;

    println!(
        "  {} Initialized wit repo at {}",
        "✓".green(),
        wit_path.display()
    );
    println!("  Run {} to start tracking a city", "wit add <location>".cyan());

    Ok(())
}
