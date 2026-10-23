// (C) 2026 - Enzo Lombardi

//! Renders `logo.png` at the repo root: a classic Turbo Vision window,
//! titled "tv-extensions" and spelling the name out in large block
//! letters, on the framework's blue desktop, with a drop shadow.
//!
//! `turbo_vision::core::screenshot::render_to_png` writes fully
//! uncompressed PNGs (stored/non-compressed DEFLATE blocks), so after
//! rendering this also re-deflates the file losslessly at the best
//! compression level — same pixels, much smaller file — using `flate2`,
//! a dev-dependency of the example only (not a runtime dependency of the
//! crate).
//!
//! Run with `cargo run --example make_logo --features screenshot`.

use std::path::Path;

use turbo_vision::core::draw::Cell;
use turbo_vision::core::palette::{Attr, TvColor};
use turbo_vision::core::screenshot::render_to_png;

#[path = "shared/png.rs"]
mod png;
use png::recompress_png;

const DESKTOP: Attr = Attr::new(TvColor::LightGray, TvColor::Blue);
const FRAME: Attr = Attr::new(TvColor::White, TvColor::Blue);
const INK: Attr = Attr::new(TvColor::Yellow, TvColor::Blue);
const TAGLINE_ATTR: Attr = Attr::new(TvColor::LightGray, TvColor::Blue);
const SHADOW: Attr = Attr::new(TvColor::Black, TvColor::Black);

const NAME: &str = "tv-extensions";
const TITLE: &str = " tv-extensions ";
const TAGLINE: &str = "extensions for turbo-vision";

/// Height of the block-letter font, in rows.
const LETTER_ROWS: usize = 5;
/// Rows of plain desktop left below the window's shadow.
const DESKTOP_BOTTOM_MARGIN: usize = 4;

/// A 5-row block font, one glyph per letter in [`NAME`], 3-4 cells wide.
/// `#` is a full block, `.` is background; a 1-column gap is inserted
/// between letters by the caller. Shapes are chosen so each letter has a
/// distinct silhouette: `t` a crossbar over a centred stem, `v` converges
/// to a point, `e`/`n`/`s`/`o` are the familiar three-bar / two-leg /
/// zig-zag / box shapes, `x` an hourglass (two triangles meeting at the
/// waist, the block-font approximation of crossing diagonals), `i` a dot
/// over a stem, `-` a single middle bar.
fn glyph(c: char) -> &'static [&'static str] {
    match c {
        't' => &["####", ".##.", ".##.", ".##.", ".##."],
        'v' => &["#.#", "#.#", "#.#", ".#.", ".#."],
        '-' => &["...", "...", "###", "...", "..."],
        'e' => &["####", "#...", "###.", "#...", "####"],
        'x' => &["#..#", ".##.", ".##.", ".##.", "#..#"],
        'n' => &["####", "#..#", "#..#", "#..#", "#..#"],
        's' => &["####", "#...", "####", "...#", "####"],
        'i' => &["##", "..", "##", "##", "##"],
        'o' => &[".##.", "#..#", "#..#", "#..#", ".##."],
        _ => unreachable!("every character in NAME has a glyph"),
    }
}

fn main() -> std::io::Result<()> {
    let glyphs: Vec<&[&str]> = NAME.chars().map(glyph).collect();
    let letters_width: usize =
        glyphs.iter().map(|g| g[0].len()).sum::<usize>() + (glyphs.len() - 1); // 1-col gaps
    let inner = letters_width.max(TAGLINE.len());
    let win_w = inner + 2; // plus the left/right border columns
    let cols = win_w + 2; // desktop margin + shadow column

    // Row layout, named explicitly to keep the arithmetic honest: a desktop
    // margin, the top border, a blank padding row, 5 rows of block letters,
    // a blank padding row, the tagline, the bottom border, the shadow row,
    // then a desktop margin below.
    let border_top = 1;
    let pad_before = border_top + 1;
    let letters_first = pad_before + 1;
    let pad_after = letters_first + LETTER_ROWS;
    let tagline_row = pad_after + 1;
    let border_bottom = tagline_row + 1;
    let shadow_row = border_bottom + 1;
    let rows = shadow_row + 1 + DESKTOP_BOTTOM_MARGIN;

    let win_left = 1;
    let win_right = win_left + win_w - 1;
    let shadow_col = win_right + 1;

    let mut buf = vec![vec![Cell::new('░', DESKTOP); cols]; rows];

    // Top border, with the title centred in the double rule.
    {
        let pad_left = (inner - TITLE.len()) / 2;
        let pad_right = inner - TITLE.len() - pad_left;
        let fill = format!("{}{}{}", "═".repeat(pad_left), TITLE, "═".repeat(pad_right));
        set_row(
            &mut buf[border_top],
            win_left,
            win_right,
            '╔',
            '╗',
            &fill,
            FRAME,
        );
    }

    // The name, in block letters, centred.
    for (r, row) in buf
        .iter_mut()
        .skip(letters_first)
        .take(LETTER_ROWS)
        .enumerate()
    {
        let mut line = String::with_capacity(inner);
        for (i, g) in glyphs.iter().enumerate() {
            line.push_str(g[r]);
            if i + 1 < glyphs.len() {
                line.push('.');
            }
        }
        let pad = (inner - line.len()) / 2;
        let padded = " ".repeat(pad) + &line;
        set_block_row(row, win_left, win_right, &padded);
    }

    // The tagline, centred.
    {
        let pad_left = (inner - TAGLINE.len()) / 2;
        let pad_right = inner - TAGLINE.len() - pad_left;
        let fill = format!(
            "{}{}{}",
            " ".repeat(pad_left),
            TAGLINE,
            " ".repeat(pad_right)
        );
        set_row(
            &mut buf[tagline_row],
            win_left,
            win_right,
            '║',
            '║',
            &fill,
            TAGLINE_ATTR,
        );
    }

    // Bottom border.
    {
        let fill = "═".repeat(inner);
        set_row(
            &mut buf[border_bottom],
            win_left,
            win_right,
            '╚',
            '╝',
            &fill,
            FRAME,
        );
    }

    // Shadow: one cell right and one down from the window.
    for row in &mut buf[(border_top + 1)..=shadow_row] {
        row[shadow_col] = Cell::new(' ', SHADOW);
    }
    for cell in &mut buf[shadow_row][(win_left + 1)..=shadow_col] {
        *cell = Cell::new(' ', SHADOW);
    }

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("logo.png");
    render_to_png(&buf, cols, rows, 2, &path)?;
    let raw_len = std::fs::metadata(&path)?.len();

    recompress_png(&path)?;
    let final_len = std::fs::metadata(&path)?.len();

    println!(
        "wrote {} ({cols}x{rows} cells, scale 2): {raw_len} bytes uncompressed, {final_len} bytes after recompression",
        path.display()
    );
    Ok(())
}

/// Set a border/text row: `lc`/`rc` are the left/right border characters,
/// `fill` supplies the `inner` literal characters between them.
fn set_row(
    row: &mut [Cell],
    left: usize,
    right: usize,
    lc: char,
    rc: char,
    fill: &str,
    attr: Attr,
) {
    row[left] = Cell::new(lc, FRAME);
    row[right] = Cell::new(rc, FRAME);
    for (i, ch) in fill.chars().enumerate() {
        row[left + 1 + i] = Cell::new(ch, attr);
    }
}

/// Set a block-letter row: `pattern`'s `#` becomes a full block, anything
/// else stays background.
fn set_block_row(row: &mut [Cell], left: usize, right: usize, pattern: &str) {
    row[left] = Cell::new('║', FRAME);
    row[right] = Cell::new('║', FRAME);
    for (i, ch) in pattern.chars().enumerate() {
        row[left + 1 + i] = Cell::new(if ch == '#' { '█' } else { ' ' }, INK);
    }
}
