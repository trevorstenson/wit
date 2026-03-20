use thiserror::Error;

#[derive(Error, Debug)]
pub enum WitError {
    #[error("wit repo not initialized — run `wit init` first")]
    NotInitialized,

    #[error("location '{0}' not found — check spelling or try a nearby city")]
    LocationNotFound(String),

    #[error("location '{0}' is already tracked")]
    LocationExists(String),

    #[error("no tracked locations — run `wit add <city>` first")]
    NoLocations,

    #[error("no snapshots yet — run `wit snap` first")]
    NoSnapshots,

    #[error("could not resolve time '{0}' — try formats like 7d, 2w, 3m, 1y, yesterday")]
    InvalidTime(String),

    #[allow(dead_code)]
    #[error("ambiguous location '{name}' — did you mean one of these?\n{options}")]
    AmbiguousLocation { name: String, options: String },

    #[error("API error: {0}")]
    Api(String),

    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, WitError>;
