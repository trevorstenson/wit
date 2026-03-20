# wit

Git-style CLI for tracking, comparing, and charting weather over time.

`wit` stores weather snapshots as TOML files inside a git repo (`~/.wit` by default). Every fetch is a commit, so you get a full diffable history of conditions across all your tracked locations. When you ask about the past, it checks git history first and falls back to the Open-Meteo historical API.

## Install

```
cargo build --release
cp target/release/wit ~/.local/bin/ # or wherever you keep binaries
```

## Getting started

```
wit init                  # create the weather repo
wit add tokyo             # track a city (geocodes automatically)
wit add boston
wit snap                  # fetch current weather for all locations, commit
```

Running `wit` with no arguments shows a status dashboard of all tracked locations.

## Quick queries

The whole point is that you don't have to remember subcommands for common stuff:

```
wit tokyo                 # current weather
wit tokyo 7d              # now vs 7 days ago
wit tokyo..boston          # compare two cities
wit tokyo jan..jul         # compare January vs July
wit tokyo..boston 1w       # compare two cities a week ago
```

Time specs: `7d`, `2w`, `3m`, `1y`, `yesterday`, month names (`jan`, `january`), years (`2024`).

## Commands

| Command | What it does |
|---------|-------------|
| `wit init [path]` | Initialize repo (defaults to `~/.wit`) |
| `wit add <city>` | Geocode and start tracking a location |
| `wit snap` | Fetch + commit weather for all tracked locations |
| `wit status [location]` | Dashboard of current conditions |
| `wit log [location] [-n N]` | Commit history |
| `wit locations` | List tracked locations with coords |
| `wit diff <args>` | Compare snapshots (same syntax as quick queries) |
| `wit backfill <location> --since <spec>` | Backfill historical data |
| `wit chart [metric] -l <locations> -r <range>` | ASCII chart over time |

### Charts

```
wit chart temp -l tokyo,boston -r 30d
wit chart humidity -r 2w
wit chart wind
```

Available metrics: `temp`, `feels`, `high`, `low`, `humidity`, `pressure`, `wind`, `gusts`, `uv`, `precip`, `cloud`.

## Configuration

Settings live in `~/.wit/wit.toml`. Right now the main thing you can change is units:

```toml
[settings]
units = "imperial"   # or "metric"
```

Locations are added via `wit add` and tracked in the same file.

## How it works

The repo layout under `~/.wit` looks like:

```
.git/
wit.toml
locations/
  tokyo/
    meta.toml        # name, coordinates, timezone
    current.toml     # latest snapshot
  boston/
    meta.toml
    current.toml
```

Each snapshot captures temperature (current, feels like, high, low), wind (speed, direction, gusts), atmosphere (humidity, pressure, cloud cover, UV), and precipitation (amount, probability, snowfall). All data comes from [Open-Meteo](https://open-meteo.com/), no API key needed.

## License

MIT
