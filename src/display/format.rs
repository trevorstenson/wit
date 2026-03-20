use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};

use crate::weather::{
    api::wind_direction_str,
    codes::describe_weather_code,
    snapshot::WeatherSnapshot,
};

pub fn print_current(location_name: &str, snap: &WeatherSnapshot, units: &str) {
    let unit = if units == "metric" { "C" } else { "F" };
    let wind_unit = if units == "metric" { "km/h" } else { "mph" };
    let (_, emoji) = describe_weather_code(snap.snapshot.weather_code);

    println!();
    println!(
        "  {} {}  {}",
        emoji,
        location_name.bold(),
        snap.snapshot
            .timestamp
            .format("%a %b %d, %I:%M %p")
            .to_string()
            .dimmed()
    );
    println!(
        "  {}",
        snap.snapshot.weather_description.italic()
    );
    println!();

    // Temperature block
    let tc = temp_color_str(snap.temperature.current, units);
    println!(
        "  Temperature  {} {}",
        format!("{:.0}{}", snap.temperature.current, unit)
            .color(tc)
            .bold(),
        format!(
            "(feels like {:.0}{})",
            snap.temperature.feels_like, unit
        )
        .dimmed()
    );
    println!(
        "               {} {:.0}{}  {} {:.0}{}",
        "H".red(),
        snap.temperature.high,
        unit,
        "L".blue(),
        snap.temperature.low,
        unit
    );
    println!();

    // Wind
    println!(
        "  Wind         {:.0} {} {}  gusts {:.0} {}",
        snap.wind.speed,
        wind_unit,
        wind_direction_str(snap.wind.direction),
        snap.wind.gusts,
        wind_unit
    );

    // Atmosphere
    println!(
        "  Humidity     {}%    Cloud cover  {}%",
        snap.atmosphere.humidity, snap.atmosphere.cloud_cover
    );
    println!(
        "  Pressure     {:.0} hPa    UV index  {:.1}",
        snap.atmosphere.pressure, snap.atmosphere.uv_index
    );

    // Precipitation
    if snap.precipitation.amount > 0.0 || snap.precipitation.probability > 0 {
        println!(
            "  Precip       {:.1} mm  ({}% chance)",
            snap.precipitation.amount, snap.precipitation.probability
        );
    }
    if snap.precipitation.snowfall > 0.0 {
        println!("  Snowfall     {:.1} cm", snap.precipitation.snowfall);
    }

    println!();
}

pub fn print_diff(
    left_name: &str,
    right_name: &str,
    left: &WeatherSnapshot,
    right: &WeatherSnapshot,
    units: &str,
) {
    let unit = if units == "metric" { "C" } else { "F" };
    let wind_unit = if units == "metric" { "km/h" } else { "mph" };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    table.set_header(vec![
        Cell::new("").fg(Color::DarkGrey),
        Cell::new(left_name).fg(Color::Cyan),
        Cell::new(right_name).fg(Color::Magenta),
        Cell::new("Delta").fg(Color::Yellow),
    ]);

    // Weather description
    table.add_row(vec![
        Cell::new("Conditions"),
        Cell::new(&left.snapshot.weather_description),
        Cell::new(&right.snapshot.weather_description),
        Cell::new(""),
    ]);

    // Temperature
    let temp_delta = right.temperature.current - left.temperature.current;
    table.add_row(vec![
        Cell::new("Temperature"),
        Cell::new(format!("{:.0}{}", left.temperature.current, unit)),
        Cell::new(format!("{:.0}{}", right.temperature.current, unit)),
        Cell::new(format_delta(temp_delta, unit)).fg(delta_color(temp_delta)),
    ]);

    // Feels like
    let feels_delta = right.temperature.feels_like - left.temperature.feels_like;
    table.add_row(vec![
        Cell::new("Feels like"),
        Cell::new(format!("{:.0}{}", left.temperature.feels_like, unit)),
        Cell::new(format!("{:.0}{}", right.temperature.feels_like, unit)),
        Cell::new(format_delta(feels_delta, unit)).fg(delta_color(feels_delta)),
    ]);

    // High/Low
    let high_delta = right.temperature.high - left.temperature.high;
    table.add_row(vec![
        Cell::new("High"),
        Cell::new(format!("{:.0}{}", left.temperature.high, unit)),
        Cell::new(format!("{:.0}{}", right.temperature.high, unit)),
        Cell::new(format_delta(high_delta, unit)).fg(delta_color(high_delta)),
    ]);

    let low_delta = right.temperature.low - left.temperature.low;
    table.add_row(vec![
        Cell::new("Low"),
        Cell::new(format!("{:.0}{}", left.temperature.low, unit)),
        Cell::new(format!("{:.0}{}", right.temperature.low, unit)),
        Cell::new(format_delta(low_delta, unit)).fg(delta_color(low_delta)),
    ]);

    // Wind
    let wind_delta = right.wind.speed - left.wind.speed;
    table.add_row(vec![
        Cell::new("Wind"),
        Cell::new(format!(
            "{:.0} {} {}",
            left.wind.speed,
            wind_unit,
            wind_direction_str(left.wind.direction)
        )),
        Cell::new(format!(
            "{:.0} {} {}",
            right.wind.speed,
            wind_unit,
            wind_direction_str(right.wind.direction)
        )),
        Cell::new(format_delta(wind_delta, wind_unit)).fg(delta_color(wind_delta)),
    ]);

    // Humidity
    let hum_delta = (right.atmosphere.humidity - left.atmosphere.humidity) as f64;
    table.add_row(vec![
        Cell::new("Humidity"),
        Cell::new(format!("{}%", left.atmosphere.humidity)),
        Cell::new(format!("{}%", right.atmosphere.humidity)),
        Cell::new(format_delta(hum_delta, "%")).fg(delta_color(hum_delta)),
    ]);

    // Pressure
    let press_delta = right.atmosphere.pressure - left.atmosphere.pressure;
    table.add_row(vec![
        Cell::new("Pressure"),
        Cell::new(format!("{:.0} hPa", left.atmosphere.pressure)),
        Cell::new(format!("{:.0} hPa", right.atmosphere.pressure)),
        Cell::new(format_delta(press_delta, " hPa")).fg(delta_color(press_delta)),
    ]);

    println!();
    println!(
        "  {} {} vs {}",
        "diff".bold(),
        left_name.cyan(),
        right_name.magenta()
    );
    println!(
        "  {}   {}",
        left.snapshot.timestamp.format("%Y-%m-%d %H:%M").to_string().dimmed(),
        right.snapshot.timestamp.format("%Y-%m-%d %H:%M").to_string().dimmed()
    );
    println!();
    println!("{table}");
    println!();
}

pub fn print_status_table(
    entries: &[(String, WeatherSnapshot)],
    units: &str,
) {
    let unit = if units == "metric" { "C" } else { "F" };
    let wind_unit = if units == "metric" { "km/h" } else { "mph" };

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);

    table.set_header(vec![
        Cell::new("Location").fg(Color::Cyan),
        Cell::new("Temp").fg(Color::Yellow),
        Cell::new("Feels").fg(Color::Yellow),
        Cell::new("H / L").fg(Color::Yellow),
        Cell::new("Conditions"),
        Cell::new("Wind"),
        Cell::new("Humidity"),
        Cell::new("Updated").fg(Color::DarkGrey),
    ]);

    for (name, snap) in entries {
        let (_, emoji) = describe_weather_code(snap.snapshot.weather_code);
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format!("{:.0}{}", snap.temperature.current, unit))
                .fg(temp_color(snap.temperature.current, units)),
            Cell::new(format!("{:.0}{}", snap.temperature.feels_like, unit)),
            Cell::new(format!(
                "{:.0}{} / {:.0}{}",
                snap.temperature.high, unit, snap.temperature.low, unit
            )),
            Cell::new(format!(
                "{} {}",
                emoji, snap.snapshot.weather_description
            )),
            Cell::new(format!(
                "{:.0} {} {}",
                snap.wind.speed,
                wind_unit,
                wind_direction_str(snap.wind.direction)
            )),
            Cell::new(format!("{}%", snap.atmosphere.humidity)),
            Cell::new(
                snap.snapshot
                    .timestamp
                    .format("%m/%d %H:%M")
                    .to_string(),
            ),
        ]);
    }

    println!();
    println!("{table}");
    println!();
}

fn format_delta(delta: f64, suffix: &str) -> String {
    if delta.abs() < 0.5 {
        "—".to_string()
    } else if delta > 0.0 {
        format!("+{:.0}{}", delta, suffix)
    } else {
        format!("{:.0}{}", delta, suffix)
    }
}

fn delta_color(delta: f64) -> Color {
    if delta.abs() < 0.5 {
        Color::DarkGrey
    } else if delta > 0.0 {
        Color::Red
    } else {
        Color::Blue
    }
}

fn temp_color(temp: f64, units: &str) -> Color {
    let f = if units == "metric" {
        temp * 9.0 / 5.0 + 32.0
    } else {
        temp
    };
    match f as i32 {
        ..=32 => Color::Blue,
        33..=50 => Color::Cyan,
        51..=70 => Color::Green,
        71..=85 => Color::Yellow,
        86..=100 => Color::Red,
        _ => Color::Magenta,
    }
}

/// Returns a color name string for the `colored` crate
fn temp_color_str(temp: f64, units: &str) -> &'static str {
    let f = if units == "metric" {
        temp * 9.0 / 5.0 + 32.0
    } else {
        temp
    };
    match f as i32 {
        ..=32 => "blue",
        33..=50 => "cyan",
        51..=70 => "green",
        71..=85 => "yellow",
        86..=100 => "red",
        _ => "magenta",
    }
}
