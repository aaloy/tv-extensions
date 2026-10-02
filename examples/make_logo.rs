// (C) 2026 - Enzo Lombardi

//! Renders `logo.png` at the repo root: a classic Turbo Vision window,
//! titled "tv-extensions" and spelling the name out in large block
//! letters, on the framework's blue desktop, with a drop shadow.
//!
//! Run with `cargo run --example make_logo --features screenshot`.

use std::path::Path;

use turbo_vision::core::draw::Cell;
use turbo_vision::core::palette::{Attr, TvColor};
use turbo_vision::core::screenshot::render_to_png;

const DESKTOP: Attr = Attr::new(TvColor::LightGray, TvColor::Blue);
const FRAME: Attr = Attr::new(TvColor::White, TvColor::Blue);
const INK: Attr = Attr::new(TvColor::Yellow, TvColor::Blue);
const TAGLINE_ATTR: Attr = Attr::new(TvColor::LightGray, TvColor::Blue);
const SHADOW: Attr = Attr::new(TvColor::Black, TvColor::Black);

/// "tv-extensions" as a 2x-oversampled bitmap: 6 sub-rows by (2 * width)
/// sub-columns per letter, `#` ink and `.` background. Each pair of
/// sub-rows/sub-columns is folded into one screen cell by [`fold`], using
/// the screenshot renderer's quadrant-block characters (`▘▝▖▗▀▄▌▐…█`) for
/// 2x2 sub-cell resolution, so a 2-or-1-column-wide letter still reads as
/// a shape rather than a solid blob.
const WORD: &[&[&str; 6]] = &[
    &["##", "##", ".#", ".#", ".#", ".#"],             // t (narrow)
    &["#..#", "#..#", "#..#", ".##.", ".##.", "..#."], // v
    &["..", "..", "##", "##", "..", ".."],             // -
    &["####", "#...", "###.", "#...", "#...", "####"], // e
    &["#..#", "#..#", ".##.", ".##.", "#..#", "#..#"], // x
    &["##", "##", ".#", ".#", ".#", ".#"],             // t (narrow)
    &["####", "#...", "###.", "#...", "#...", "####"], // e
    &["#..#", "##.#", "#.##", "#..#", "#..#", "#..#"], // n
    &["####", "#...", "####", "...#", "...#", "####"], // s
    &["##", "..", "##", "##", "##", "##"],              // i
    &[".##.", "#..#", "#..#", "#..#", "#..#", ".##."], // o
    &["#..#", "##.#", "#.##", "#..#", "#..#", "#..#"], // n
    &["####", "#...", "####", "...#", "...#", "####"], // s
];

/// Whether a 1-column gap separates `WORD[i]` from `WORD[i + 1]`; some are
/// dropped (around the dash, and after `o`) to keep the word inside the
/// window's interior width.
const GAP_AFTER: [bool; 12] = [
    true, false, false, true, true, true, true, true, true, true, false, true,
];

/// Fold a 2x-oversampled bitmap row pair into one screen-cell row, using
/// quadrant-block characters for 2x2 sub-cell resolution.
fn fold(bitmap: &[&str; 6]) -> [String; 3] {
    let at = |row: &str, col: usize| row.as_bytes()[col] == b'#';
    std::array::from_fn(|r| {
        let (top, bottom) = (bitmap[2 * r], bitmap[2 * r + 1]);
        (0..top.len() / 2)
            .map(|c| quad_char(at(top, 2 * c), at(top, 2 * c + 1), at(bottom, 2 * c), at(bottom, 2 * c + 1)))
            .collect()
    })
}

/// The quadrant-block character for a 2x2 pattern of on/off sub-cells.
#[allow(
    clippy::fn_params_excessive_bools,
    reason = "Each bool is an independent quadrant of one glyph cell, not a cluster of flags."
)]
fn quad_char(ul: bool, ur: bool, ll: bool, lr: bool) -> char {
    match (ul, ur, ll, lr) {
        (false, false, false, false) => ' ',
        (true, false, false, false) => '▘',
        (false, true, false, false) => '▝',
        (false, false, true, false) => '▖',
        (false, false, false, true) => '▗',
        (true, true, false, false) => '▀',
        (false, false, true, true) => '▄',
        (true, false, true, false) => '▌',
        (false, true, false, true) => '▐',
        (true, false, false, true) => '▚',
        (false, true, true, false) => '▞',
        (true, true, true, false) => '▛',
        (true, true, false, true) => '▜',
        (true, false, true, true) => '▙',
        (false, true, true, true) => '▟',
        (true, true, true, true) => '█',
    }
}

const TITLE: &str = " tv-extensions ";
const TAGLINE: &str = "extensions for turbo-vision";

fn main() -> std::io::Result<()> {
    let folded: Vec<[String; 3]> = WORD.iter().map(|g| fold(g)).collect();
    let letters_width: usize = folded.iter().map(|g| g[0].chars().count()).sum::<usize>()
        + GAP_AFTER.iter().filter(|&&gap| gap).count();
    let inner = letters_width.max(TAGLINE.len());
    let win_w = inner + 2; // plus the left/right border columns
    let cols = win_w + 3; // desktop margin + shadow column + desktop margin
    let rows = 7; // top border, 3 letter rows, tagline, bottom border, shadow

    let win_left = 1;
    let win_right = win_left + win_w - 1;
    let shadow_col = win_right + 1;

    let mut buf = vec![vec![Cell::new('░', DESKTOP); cols]; rows];

    // Row 0: top border, with the title centred in the double rule.
    {
        let pad_left = (inner - TITLE.len()) / 2;
        let pad_right = inner - TITLE.len() - pad_left;
        let fill = format!(
            "{}{}{}",
            "═".repeat(pad_left),
            TITLE,
            "═".repeat(pad_right)
        );
        set_row(&mut buf[0], win_left, win_right, '╔', '╗', &fill, FRAME);
    }

    // Rows 1..=3: the name, in block letters.
    for (r, row) in buf.iter_mut().skip(1).take(3).enumerate() {
        let mut line = String::with_capacity(inner);
        for (i, glyph) in folded.iter().enumerate() {
            line.push_str(&glyph[r]);
            if GAP_AFTER.get(i).copied().unwrap_or(false) {
                line.push(' ');
            }
        }
        let pad = (inner - line.chars().count()) / 2;
        let padded: String = " ".repeat(pad) + &line;
        set_block_row(row, win_left, win_right, &padded);
    }

    // Row 4: the tagline, centred.
    {
        let pad_left = (inner - TAGLINE.len()) / 2;
        let pad_right = inner - TAGLINE.len() - pad_left;
        let fill = format!(
            "{}{}{}",
            " ".repeat(pad_left),
            TAGLINE,
            " ".repeat(pad_right)
        );
        set_row(&mut buf[4], win_left, win_right, '║', '║', &fill, TAGLINE_ATTR);
    }

    // Row 5: bottom border.
    {
        let fill = "═".repeat(inner);
        set_row(&mut buf[5], win_left, win_right, '╚', '╝', &fill, FRAME);
    }

    // Shadow: one cell right and one down from the window.
    for row in buf.iter_mut().skip(1).take(rows - 1) {
        row[shadow_col] = Cell::new(' ', SHADOW);
    }
    let last = rows - 1;
    for cell in &mut buf[last][(win_left + 1)..=shadow_col] {
        *cell = Cell::new(' ', SHADOW);
    }

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("logo.png");
    render_to_png(&buf, cols, rows, 1, &path)?;
    println!("wrote {} ({cols}x{rows} cells, scale 1)", path.display());
    Ok(())
}

/// Set a border/text row: `lc`/`rc` are the left/right border characters,
/// `fill` supplies the `inner` literal characters between them.
fn set_row(row: &mut [Cell], left: usize, right: usize, lc: char, rc: char, fill: &str, attr: Attr) {
    row[left] = Cell::new(lc, FRAME);
    row[right] = Cell::new(rc, FRAME);
    for (i, ch) in fill.chars().enumerate() {
        row[left + 1 + i] = Cell::new(ch, attr);
    }
}

/// Set a block-letter row: `pattern` already holds the final quadrant-block
/// (or space) character for each cell.
fn set_block_row(row: &mut [Cell], left: usize, right: usize, pattern: &str) {
    row[left] = Cell::new('║', FRAME);
    row[right] = Cell::new('║', FRAME);
    for (i, ch) in pattern.chars().enumerate() {
        row[left + 1 + i] = Cell::new(ch, INK);
    }
}
