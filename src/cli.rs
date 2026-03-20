use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "wit",
    about = "Version-controlled weather journal",
    long_about = "wit — git semantics for weather data. Track, compare, and chart weather history.",
    args_conflicts_with_subcommands = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Quick weather query: wit tokyo, wit tokyo 7d, wit tokyo..boston
    pub query: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new wit repo
    Init {
        /// Path for the wit repo (default: ~/.wit)
        path: Option<String>,
    },

    /// Track a new location
    Add {
        /// City name to geocode and track
        location: String,
    },

    /// Fetch weather for all tracked locations and commit
    Snap,

    /// Alias for snap
    Fetch,

    /// Show weather history
    Log {
        /// Filter to a specific location
        location: Option<String>,

        /// Number of entries to show
        #[arg(short, default_value = "10")]
        n: usize,
    },

    /// Dashboard of current conditions
    Status {
        /// Filter to a specific location
        location: Option<String>,
    },

    /// List tracked locations
    Locations,

    /// Compare weather snapshots
    Diff {
        /// Comparison spec: location..location, location <time>, or location time..time
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
    },

    /// Backfill historical weather data for a tracked location
    Backfill {
        /// Location to backfill
        location: String,

        /// How far back: 30d, 3m, 1y, or a date like 2025-01-01
        #[arg(long)]
        since: String,
    },

    /// ASCII chart of weather metrics over time
    Chart {
        /// Metric to chart: temp, humidity, wind, pressure
        metric: Option<String>,

        /// Locations to chart (comma-separated, omit for all tracked)
        #[arg(short, long, value_delimiter = ',', num_args = 1..)]
        location: Vec<String>,

        /// Time range (e.g., 30d, 3m)
        #[arg(short, long, default_value = "30d")]
        range: String,
    },
}
