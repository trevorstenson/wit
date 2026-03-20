# wit

<p align="center"><strong>Git for weather.</strong></p>

<p align="center">Track cities, commit snapshots, diff seasons, and chart weather history from your terminal.</p>

<p align="center"><code>no API key</code> • <code>git-backed history</code> • <code>fast query syntax</code></p>

<p align="center">
  <img src="assets/chart_sample.png" alt="wit ASCII weather chart" width="860">
</p>

```bash
wit tokyo
wit tokyo 7d
wit tokyo..boston
wit tokyo jan..jul
wit backfill tokyo --since 1y
```

`wit` turns weather into something you can inspect like code: snapshot it, store it in git, diff it later, and chart how it changed.

## Why It Grabs Attention

- Every tracked weather snapshot is committed to a local git repo.
- Historical backfill writes real backdated commits, so the timeline is queryable.
- Quick queries work for both tracked locations and one-off city lookups.
- The output is built for terminals: dashboards, side-by-side diffs, and ASCII charts.
- It runs on Open-Meteo, so there is no API key or account setup.

## Quick Start

```bash
wit init
wit add tokyo
wit add boston
wit snap
wit
wit diff tokyo..boston
wit chart temp -l tokyo,boston -r 30d
```

What that gives you:

- `wit` with no arguments shows a dashboard across tracked locations.
- `wit snap` fetches current weather and commits the result.
- `wit diff` compares locations or points in time in a table.
- `wit chart` renders terminal charts from git history.

## Query Syntax

The shorthand query language is the main feature.

```bash
wit tokyo                  # current weather for any city
wit tokyo 7d               # now vs 7 days ago
wit tokyo..boston          # compare two cities now
wit tokyo..boston 1w       # compare two cities a week ago
wit tokyo jan..jul         # compare one city across seasons
wit tokyo 2024             # compare now vs the same date in 2024
```

Time specs supported:

- `7d`, `2w`, `3m`, `1y`
- `yesterday`, `last-week`, `last-month`
- month names like `jan`, `january`
- years like `2024`

## What It Looks Like

Running `wit` with no arguments shows a dashboard across everything you're tracking:

```text
╭──────────────────────────────────────┬──────┬───────┬───────────┬────────────────────┬──────────┬──────────┬─────────────╮
│ Location                             ┆ Temp ┆ Feels ┆ H / L     ┆ Conditions         ┆ Wind     ┆ Humidity ┆ Updated     │
╞══════════════════════════════════════╪══════╪═══════╪═══════════╪════════════════════╪══════════╪══════════╪═════════════╡
│ New York, New York, United States    ┆ 35F  ┆ 35F   ┆ 39F / 31F ┆ 🌦 Moderate drizzle ┆ 5 mph SW ┆ 95%      ┆ 02/18 12:00 │
│ Tokyo, Tokyo, Japan                  ┆ 44F  ┆ 48F   ┆ 53F / 35F ┆ 🌦 Light drizzle    ┆ 14 mph N ┆ 49%      ┆ 02/18 12:00 │
│ Boston, Massachusetts, United States ┆ 52F  ┆ 42F   ┆ 56F / 31F ┆ 🌥 Overcast         ┆ 17 mph S ┆ 46%      ┆ 03/20 16:43 │
╰──────────────────────────────────────┴──────┴───────┴───────────┴────────────────────┴──────────┴──────────┴─────────────╯
```

Compare locations side by side:

```text
$ wit diff tokyo..boston 21d

╭─────────────┬──────────────────────────────────┬───────────────────────────────────────────────────┬────────╮
│             ┆ Tokyo, Tokyo, Japan (2026-02-27) ┆ Boston, Massachusetts, United States (2026-02-27) ┆ Delta  │
╞═════════════╪══════════════════════════════════╪═══════════════════════════════════════════════════╪════════╡
│ Temperature ┆ 48F                              ┆ 31F                                               ┆ -17F   │
│ Feels like  ┆ 44F                              ┆ 25F                                               ┆ -19F   │
│ Wind        ┆ 2 mph E                          ┆ 4 mph SE                                          ┆ +2mph  │
│ Humidity    ┆ 47%                              ┆ 81%                                               ┆ +34%   │
╰─────────────┴──────────────────────────────────┴───────────────────────────────────────────────────┴────────╯
```

## Install

```bash
cargo install --path .
```

## Getting Started

```bash
wit init
wit add tokyo
wit add boston
wit snap
```

After that, run `wit snap` whenever you want a new checkpoint, or automate it with cron/launchd/systemd. The value of the tool compounds as the git history grows.

## Commands

| Command | What it does |
|---------|-------------|
| `wit init [path]` | Initialize the weather repo, defaulting to `~/.wit` |
| `wit add <city>` | Geocode and start tracking a location |
| `wit snap` | Fetch and commit weather for all tracked locations |
| `wit status [location]` | Show the current dashboard |
| `wit log [location] [-n N]` | Show snapshot history from git |
| `wit locations` | List tracked locations with coordinates and timezone |
| `wit diff <args>` | Compare snapshots using the query syntax |
| `wit backfill <location> --since <spec>` | Import historical data into git history |
| `wit chart [metric] -l <locations> -r <range>` | Render an ASCII chart over time |

## Metrics

`wit chart` supports:

`temp`, `feels`, `high`, `low`, `humidity`, `pressure`, `wind`, `gusts`, `uv`, `precip`, `cloud`

## How It Works

The repo layout under `~/.wit` looks like this:

```text
.git/
wit.toml
locations/
  tokyo/
    meta.toml
    current.toml
  boston/
    meta.toml
    current.toml
```

Tracked locations live in `wit.toml`. Each location has metadata plus its latest snapshot on disk. Git stores the history, and `wit` reads older snapshots back out of commits when you run time-based queries or charts.

## Configuration

Settings live in `~/.wit/wit.toml`.

```toml
[settings]
units = "imperial" # or "metric"
```

## Data Source

Weather data comes from [Open-Meteo](https://open-meteo.com/). Current snapshots include richer live fields like cloud cover and precipitation probability. Historical backfill uses daily archive data, so some fields are necessarily coarser.

## Caveats

- `wit` is strongest once you have a bit of history; the first day is less interesting than day 30.
- Historical queries for tracked locations use git history when available and the archive API when needed.
- This repo currently builds cleanly with `cargo check`, but there are no automated tests yet.

## License

MIT
