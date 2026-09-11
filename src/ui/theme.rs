use ratatui::style::Color;

pub const BG: (u8, u8, u8) = (20, 14, 38);
pub const BG_DIM: (u8, u8, u8) = (14, 10, 28);
/// Bottom of the vertical background gradient painted behind the waves.
pub const BG_DEEP: (u8, u8, u8) = (10, 8, 22);

pub const PINK: (u8, u8, u8) = (255, 140, 205);
pub const LAVENDER: (u8, u8, u8) = (185, 150, 255);
pub const CYAN: (u8, u8, u8) = (110, 235, 225);
pub const PEACH: (u8, u8, u8) = (255, 185, 140);
pub const GOLD: (u8, u8, u8) = (255, 225, 150);
pub const PALE: (u8, u8, u8) = (235, 228, 255);
pub const FOG_GREY: (u8, u8, u8) = (150, 142, 180);
pub const STORM_FLASH: (u8, u8, u8) = (255, 250, 220);
pub const MUTED_TEXT: (u8, u8, u8) = (150, 140, 190);

/// The signature wave gradient: pink -> lavender -> cyan -> peach -> back to pink.
pub const WAVE_STOPS: [(u8, u8, u8); 4] = [PINK, LAVENDER, CYAN, PEACH];

pub fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

pub fn lerp(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| -> u8 { (x as f64 + (y as f64 - x as f64) * t).round() as u8 };
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

pub fn dim(c: (u8, u8, u8), factor: f64) -> (u8, u8, u8) {
    let f = factor.clamp(0.0, 1.0);
    (
        (c.0 as f64 * f).round() as u8,
        (c.1 as f64 * f).round() as u8,
        (c.2 as f64 * f).round() as u8,
    )
}

/// Sample the repeating wave gradient at an arbitrary position `t` (wraps every 1.0).
pub fn wave_color(t: f64) -> (u8, u8, u8) {
    let stops = WAVE_STOPS;
    let n = stops.len();
    let t = t.rem_euclid(1.0) * n as f64;
    let i = t.floor() as usize % n;
    let j = (i + 1) % n;
    let frac = t - t.floor();
    lerp(stops[i], stops[j], frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        assert_eq!(lerp((0, 0, 0), (100, 200, 50), 0.0), (0, 0, 0));
        assert_eq!(lerp((0, 0, 0), (100, 200, 50), 1.0), (100, 200, 50));
    }

    #[test]
    fn lerp_midpoint() {
        assert_eq!(lerp((0, 0, 0), (100, 100, 100), 0.5), (50, 50, 50));
    }

    #[test]
    fn wave_color_wraps_seamlessly() {
        let a = wave_color(0.0);
        let b = wave_color(1.0);
        assert_eq!(a, b);
    }

    #[test]
    fn dim_scales_down() {
        assert_eq!(dim((100, 100, 100), 0.5), (50, 50, 50));
        assert_eq!(dim((100, 100, 100), 1.0), (100, 100, 100));
    }
}
