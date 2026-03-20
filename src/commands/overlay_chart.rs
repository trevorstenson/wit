use std::collections::HashMap;

use rgb::RGB8;

/// Braille sub-pixel bit layout (same as drawille).
/// Each braille character is a 2×4 grid of dots.
const PIXEL_MAP: [[u8; 2]; 4] = [
    [0x01, 0x08],
    [0x02, 0x10],
    [0x04, 0x20],
    [0x40, 0x80],
];
const BRAILLE_OFFSET: u32 = 0x2800;

/// Per-series bitmask in a single braille cell.
struct CellEntry {
    bitmask: u8,
    series_idx: usize,
}

/// A braille canvas that tracks which series owns each sub-pixel,
/// so overlapping series get distinct colors.
struct OverlayCanvas {
    /// (row, col) -> list of per-series bitmasks
    cells: HashMap<(u16, u16), Vec<CellEntry>>,
}

impl OverlayCanvas {
    fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    /// Set a single sub-pixel at (x, y) for the given series.
    fn set(&mut self, x: i32, y: i32, series_idx: usize) {
        if x < 0 || y < 0 {
            return;
        }
        let col = (x / 2) as u16;
        let row = (y / 4) as u16;
        let bit = PIXEL_MAP[(y % 4) as usize][(x % 2) as usize];

        let entries = self.cells.entry((row, col)).or_default();
        if let Some(entry) = entries.iter_mut().find(|e| e.series_idx == series_idx) {
            entry.bitmask |= bit;
        } else {
            entries.push(CellEntry {
                bitmask: bit,
                series_idx,
            });
        }
    }

    /// Draw a line from (x1,y1) to (x2,y2) using Bresenham's algorithm.
    fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, series_idx: usize) {
        let dx = (x2 - x1).abs();
        let dy = -(y2 - y1).abs();
        let sx: i32 = if x1 < x2 { 1 } else { -1 };
        let sy: i32 = if y1 < y2 { 1 } else { -1 };
        let mut err = dx + dy;
        let mut cx = x1;
        let mut cy = y1;

        loop {
            self.set(cx, cy, series_idx);
            if cx == x2 && cy == y2 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
    }

    /// Render the canvas to colored strings.
    /// Each cell picks the color of the series owning the most sub-pixels.
    fn render(&self, colors: &[RGB8], width_cells: u16, height_cells: u16) -> Vec<String> {
        let mut rows: Vec<String> = Vec::with_capacity(height_cells as usize);

        for row in 0..height_cells {
            let mut line = String::new();
            for col in 0..width_cells {
                if let Some(entries) = self.cells.get(&(row, col)) {
                    // Merge all bitmasks into one braille char
                    let mut combined: u8 = 0;
                    for e in entries {
                        combined |= e.bitmask;
                    }
                    let ch = char::from_u32(BRAILLE_OFFSET + combined as u32).unwrap_or(' ');

                    // Pick color of series with most sub-pixels in this cell
                    let best = entries
                        .iter()
                        .max_by_key(|e| e.bitmask.count_ones())
                        .unwrap();
                    let c = colors[best.series_idx % colors.len()];
                    line.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", c.r, c.g, c.b, ch));
                } else {
                    line.push(' ');
                }
            }
            rows.push(line);
        }

        rows
    }
}

/// A data series to plot.
pub struct SeriesData {
    pub points: Vec<(f32, f32)>,
}

/// Render an overlay chart with multiple series on the same axes.
///
/// - `series`: slice of series data (each with points as (x, y))
/// - `colors`: color for each series
/// - `y_min`, `y_max`: shared y-axis bounds
/// - `x_max`: max x value
/// - `width_chars`, `height_chars`: chart size in terminal characters
pub fn render_chart(
    series: &[SeriesData],
    colors: &[RGB8],
    y_min: f32,
    y_max: f32,
    x_max: f32,
    width_chars: u16,
    height_chars: u16,
) -> Vec<String> {
    // Sub-pixel dimensions (each char is 2 wide × 4 tall in braille dots)
    let px_w = (width_chars as i32) * 2;
    let px_h = (height_chars as i32) * 4;

    let mut canvas = OverlayCanvas::new();

    let y_range = y_max - y_min;
    let x_range = if x_max > 0.0 { x_max } else { 1.0 };

    // Draw each series
    for (si, s) in series.iter().enumerate() {
        if s.points.len() < 2 {
            continue;
        }
        for win in s.points.windows(2) {
            let (x1, y1) = win[0];
            let (x2, y2) = win[1];

            // Map data coords to pixel coords
            let px1 = ((x1 / x_range) * (px_w - 1) as f32) as i32;
            let py1 = ((1.0 - (y1 - y_min) / y_range) * (px_h - 1) as f32) as i32;
            let px2 = ((x2 / x_range) * (px_w - 1) as f32) as i32;
            let py2 = ((1.0 - (y2 - y_min) / y_range) * (px_h - 1) as f32) as i32;

            canvas.line(px1, py1, px2, py2, si);
        }
    }

    // Render to colored strings
    let rows = canvas.render(colors, width_chars, height_chars);

    // Build output with y-axis labels
    let label_width = 8;
    let mut output: Vec<String> = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        let label = if i == 0 {
            format!("{:>7.1} ", y_max)
        } else if i == rows.len() - 1 {
            format!("{:>7.1} ", y_min)
        } else if i == rows.len() / 2 {
            let mid = (y_min + y_max) / 2.0;
            format!("{:>7.1} ", mid)
        } else {
            " ".repeat(label_width)
        };
        output.push(format!("  \x1b[2m{}\x1b[0m{}", label, row));
    }

    // X-axis tick marks
    let x_axis_pad = " ".repeat(label_width + 2); // 2 for leading indent
    output.push(format!(
        "{}\x1b[2m{:<w$}{}\x1b[0m",
        x_axis_pad,
        "0",
        format!("{:.0}", x_max),
        w = width_chars as usize - format!("{:.0}", x_max).len()
    ));

    output
}
