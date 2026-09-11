/// A tiny hand-rolled 5x5 dot-matrix font, rendered as blocky ASCII art.
/// Each glyph is 5 rows of a 5-character string; '#' = filled pixel, '.' = empty.
pub const GLYPH_ROWS: usize = 5;
pub const GLYPH_COLS: usize = 5;

pub fn glyph(c: char) -> [&'static str; GLYPH_ROWS] {
    match c.to_ascii_uppercase() {
        '0' => ["#####", "#...#", "#...#", "#...#", "#####"],
        '1' => ["..#..", ".##..", "..#..", "..#..", ".###."],
        '2' => ["####.", "....#", "..##.", ".#...", "#####"],
        '3' => ["####.", "....#", "..##.", "....#", "####."],
        '4' => ["#..#.", "#..#.", "#####", "...#.", "...#."],
        '5' => ["#####", "#....", "####.", "....#", "####."],
        '6' => ["..##.", ".#...", "####.", "#...#", ".###."],
        '7' => ["#####", "...#.", "..#..", ".#...", ".#..."],
        '8' => [".###.", "#...#", ".###.", "#...#", ".###."],
        '9' => [".###.", "#...#", ".####", "....#", ".###."],
        '°' => [".##..", "#..#.", ".##..", ".....", "....."],
        ':' => [".....", "..#..", ".....", "..#..", "....."],
        '-' => [".....", ".....", "#####", ".....", "....."],
        'A' => ["..#..", ".#.#.", "#####", "#...#", "#...#"],
        'C' => [".####", "#....", "#....", "#....", ".####"],
        'D' => ["####.", "#...#", "#...#", "#...#", "####."],
        'E' => ["#####", "#....", "####.", "#....", "#####"],
        'F' => ["#####", "#....", "####.", "#....", "#...."],
        'G' => [".####", "#....", "#.###", "#...#", ".###."],
        'H' => ["#...#", "#...#", "#####", "#...#", "#...#"],
        'I' => ["#####", "..#..", "..#..", "..#..", "#####"],
        'L' => ["#....", "#....", "#....", "#....", "#####"],
        'N' => ["#...#", "##..#", "#.#.#", "#..##", "#...#"],
        'O' => [".###.", "#...#", "#...#", "#...#", ".###."],
        'R' => ["####.", "#...#", "####.", "#..#.", "#...#"],
        'S' => [".####", "#....", ".###.", "....#", "####."],
        'T' => ["#####", "..#..", "..#..", "..#..", "..#.."],
        'U' => ["#...#", "#...#", "#...#", "#...#", ".###."],
        'V' => ["#...#", "#...#", "#...#", ".#.#.", "..#.."],
        'W' => ["#...#", "#...#", "#.#.#", "##.##", "#...#"],
        'Y' => ["#...#", ".#.#.", "..#..", "..#..", "..#.."],
        'Z' => ["#####", "...#.", "..#..", ".#...", "#####"],
        _ => [".....", ".....", ".....", ".....", "....."],
    }
}

/// Width in font-columns of a single character including trailing spacing.
pub fn glyph_advance() -> usize {
    GLYPH_COLS + 1
}

/// The font-column range actually covered by ink when `text` is laid out at
/// `glyph_advance()` spacing, as `(first, last_exclusive)`. `None` if nothing is inked.
///
/// Centering on the plain advance box shifts short strings visibly to the left: the
/// box includes the trailing inter-glyph gap, and glyphs such as '°' leave their
/// right-hand columns empty. Centering on these bounds puts the visible ink in the middle.
pub fn ink_bounds(text: &str) -> Option<(usize, usize)> {
    bounds_of(text, |_| true)
}

/// The horizontal center of `text`, in font-columns, placed where the run *looks*
/// centered rather than where it measures centered.
///
/// '°' is a small superscript mark, so the eye weighs it far less than a digit and a
/// run balanced on its full ink still reads as sitting too far left. Counting the
/// degree sign at half weight puts the numerals where they look centered while it
/// still balances the right-hand side.
pub fn optical_center(text: &str) -> Option<f64> {
    let (first, last) = ink_bounds(text)?;
    let full = (first + last) as f64 / 2.0;
    match bounds_of(text, |c| c != '\u{b0}') {
        // Nothing but degree signs: there is no numeral mass to favour.
        None => Some(full),
        Some((df, dl)) => {
            let without_degree = (df + dl) as f64 / 2.0;
            Some((full + without_degree) / 2.0)
        }
    }
}

/// Ink bounds over the characters `keep` accepts, laid out at their real positions
/// in the full string so the skipped glyphs still occupy their advance.
fn bounds_of(text: &str, keep: impl Fn(char) -> bool) -> Option<(usize, usize)> {
    let advance = glyph_advance();
    let mut first = usize::MAX;
    let mut last = 0usize;
    for (i, c) in text.chars().enumerate() {
        if !keep(c) {
            continue;
        }
        for row in glyph(c) {
            for (col, pixel) in row.chars().enumerate() {
                if pixel == '#' {
                    first = first.min(i * advance + col);
                    last = last.max(i * advance + col + 1);
                }
            }
        }
    }
    (first != usize::MAX).then_some((first, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_row_has_expected_width() {
        for c in "0123456789°:-ACDEFGHILNORSTUVWYZ".chars() {
            for row in glyph(c) {
                assert_eq!(row.len(), GLYPH_COLS, "glyph '{c}' row width mismatch");
            }
        }
    }

    #[test]
    fn unknown_char_is_blank() {
        for row in glyph('#') {
            assert!(row.chars().all(|p| p == '.'));
        }
    }

    #[test]
    fn optical_center_favours_the_numerals_over_the_degree_sign() {
        // "21°" inks columns 0..16, so it measures centered on 8.0 — but the eye
        // reads that as left-shifted, so the degree sign only counts half.
        assert_eq!(ink_bounds("21°"), Some((0, 16)));
        assert_eq!(optical_center("21°"), Some(6.5));
        // With no degree sign there is nothing to discount.
        assert_eq!(optical_center("21"), Some(5.0));
        // A lone degree sign has no numerals to favour, so it just centers on its ink.
        assert_eq!(optical_center("°"), Some(2.0));
        assert_eq!(optical_center(""), None);
    }

    #[test]
    fn ink_bounds_trim_trailing_gap_and_empty_columns() {
        // "21°": '2' inks from column 0, '°' ends at column 3 of the third cell.
        assert_eq!(ink_bounds("21°"), Some((0, 2 * glyph_advance() + 4)));
        // '1' is inset on both sides, so a lone digit reports its own narrow span.
        assert_eq!(ink_bounds("1"), Some((1, 4)));
        assert_eq!(ink_bounds(""), None);
        assert_eq!(ink_bounds(" "), None);
    }
}
