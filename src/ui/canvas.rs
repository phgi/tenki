use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::net::model::Condition;
use crate::ui::bigfont::{glyph, glyph_advance, ink_bounds, GLYPH_ROWS};
use crate::ui::particles::Particle;
use crate::ui::theme::{self, dim, rgb, wave_color};

pub struct Canvas<'a> {
    pub condition: Condition,
    pub is_day: bool,
    pub hero_text: String,
    pub sub_label: &'static str,
    pub icon: &'static str,
    pub wave_phase: f64,
    pub flash: f64,
    pub particles: &'a [Particle],
}

impl<'a> Widget for Canvas<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.paint_background(area, buf);
        self.paint_waves(area, buf);
        self.paint_hero(area, buf);
        // Particles last so rain/snow drifts *in front of* the hero numerals.
        self.paint_particles(area, buf);
    }
}

impl<'a> Canvas<'a> {
    fn paint_background(&self, area: Rect, buf: &mut Buffer) {
        for y in area.top()..area.bottom() {
            let t = (y - area.top()) as f64 / area.height.max(1) as f64;
            let mut bg = theme_lerp_bg(t);
            if self.flash > 0.0 {
                bg = theme::lerp(bg, theme::STORM_FLASH, self.flash * 0.5);
            }
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_bg(rgb(bg));
                }
            }
        }
    }

    fn paint_waves(&self, area: Rect, buf: &mut Buffer) {
        let waves = 4usize;
        let spacing = area.height as f64 / (waves as f64 + 1.0);
        for i in 0..waves {
            let baseline = spacing * (i as f64 + 1.0);
            let amplitude = (spacing * 0.35).max(1.0);
            let freq = 0.12 + i as f64 * 0.015;
            let phase = self.wave_phase * (1.0 + i as f64 * 0.2) + i as f64 * 1.7;
            let hue = (i as f64 / waves as f64 + self.wave_phase * 0.05).rem_euclid(1.0);
            let mut color = wave_color(hue);
            if self.flash > 0.0 {
                color = theme::lerp(color, theme::STORM_FLASH, self.flash);
            }

            for col in 0..area.width {
                let x = area.left() + col;
                let offset = amplitude * ((col as f64) * freq + phase).sin();
                let fy = baseline + offset;
                let y = area.top() as f64 + fy;
                if y < area.top() as f64 || y >= area.bottom() as f64 {
                    continue;
                }
                let y = y as u16;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('~');
                    cell.set_fg(rgb(color));
                }
            }
        }
    }

    fn paint_particles(&self, area: Rect, buf: &mut Buffer) {
        let base = particle_color(self.condition);
        for p in self.particles {
            if p.glyph == ' ' {
                continue;
            }
            let x = area.left() as f64 + p.x;
            let y = area.top() as f64 + p.y;
            if x < area.left() as f64 || x >= area.right() as f64 {
                continue;
            }
            if y < area.top() as f64 || y >= area.bottom() as f64 {
                continue;
            }
            let color = dim(base, p.brightness);
            if let Some(cell) = buf.cell_mut((x as u16, y as u16)) {
                cell.set_char(p.glyph);
                cell.set_fg(rgb(color));
            }
        }
    }

    fn paint_hero(&self, area: Rect, buf: &mut Buffer) {
        let text = self.hero_text.to_ascii_uppercase();
        let advance = glyph_advance();
        let text_cols = text.chars().count() * advance;

        let mut scale: u16 = 4;
        while scale > 1
            && (text_cols as u16 * scale > area.width.saturating_sub(4)
                || (GLYPH_ROWS as u16 * scale) > area.height / 2)
        {
            scale -= 1;
        }

        // Center on the ink rather than the advance box, otherwise the trailing
        // inter-glyph gap (and the half-empty '°' cell) pulls the hero left.
        let (ink_first, ink_last) = ink_bounds(&text).unwrap_or((0, text_cols));
        let ink_width = ((ink_last - ink_first) as u16 * scale).min(area.width);
        let total_height = GLYPH_ROWS as u16 * scale;
        let ink_left = area.left() + area.width.saturating_sub(ink_width) / 2;
        let start_x = ink_left
            .saturating_sub(ink_first as u16 * scale)
            .max(area.left());
        let start_y = area.top() + (area.height.saturating_sub(total_height)) * 2 / 5;

        let (stop_a, stop_b) = hero_gradient(self.condition, self.is_day);

        // Waves running through 5x5 glyphs destroy legibility on small terminals,
        // so clear a padded plate behind the hero block and its label.
        let label_row = start_y + total_height + 1;
        let plate_top = start_y.saturating_sub(1);
        let plate_bottom = (label_row + 1).min(area.bottom());
        let plate_left = ink_left.saturating_sub(2).max(area.left());
        let plate_right = (ink_left + ink_width + 2).min(area.right());
        for y in plate_top..plate_bottom {
            let t = (y - area.top()) as f64 / area.height.max(1) as f64;
            let mut bg = theme_lerp_bg(t);
            if self.flash > 0.0 {
                bg = theme::lerp(bg, theme::STORM_FLASH, self.flash * 0.5);
            }
            for x in plate_left..plate_right {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_bg(rgb(bg));
                }
            }
        }

        for (ci, ch) in text.chars().enumerate() {
            let bitmap = glyph(ch);
            let glyph_x = start_x + (ci as u16) * (advance as u16) * scale;
            let t = if text.chars().count() > 1 {
                ci as f64 / (text.chars().count() - 1) as f64
            } else {
                0.0
            };
            let mut color = theme::lerp(stop_a, stop_b, t);
            if self.flash > 0.0 {
                color = theme::lerp(color, theme::STORM_FLASH, self.flash * 0.7);
            }

            for (row, line) in bitmap.iter().enumerate() {
                for (col, pixel) in line.chars().enumerate() {
                    if pixel != '#' {
                        continue;
                    }
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let x = glyph_x + (col as u16) * scale + sx;
                            let y = start_y + (row as u16) * scale + sy;
                            if x >= area.right() || y >= area.bottom() {
                                continue;
                            }
                            if let Some(cell) = buf.cell_mut((x, y)) {
                                cell.set_char('█');
                                cell.set_fg(rgb(color));
                            }
                        }
                    }
                }
            }
        }

        // Icon + condition label beneath the big hero glyph.
        let label = format!("{} {}", self.icon, self.sub_label);
        let label_x = area.left() + area.width.saturating_sub(label.chars().count() as u16) / 2;
        let label_y = label_row;
        if label_y < area.bottom() {
            for (i, ch) in label.chars().enumerate() {
                let x = label_x + i as u16;
                if x >= area.right() {
                    break;
                }
                if let Some(cell) = buf.cell_mut((x, label_y)) {
                    cell.set_char(ch);
                    cell.set_fg(rgb(theme::PALE));
                }
            }
        }
    }
}

fn theme_lerp_bg(t: f64) -> (u8, u8, u8) {
    theme::lerp(theme::BG, theme::BG_DEEP, t)
}

fn particle_color(condition: Condition) -> (u8, u8, u8) {
    match condition {
        Condition::Rain | Condition::RainShowers | Condition::Freezing => theme::CYAN,
        Condition::Drizzle => theme::LAVENDER,
        Condition::Snow | Condition::SnowShowers => theme::PALE,
        Condition::Fog => theme::FOG_GREY,
        Condition::Thunderstorm => theme::PALE,
        Condition::Clear | Condition::PartlyCloudy | Condition::Overcast => theme::PALE,
    }
}

fn hero_gradient(condition: Condition, is_day: bool) -> ((u8, u8, u8), (u8, u8, u8)) {
    match condition {
        Condition::Clear if is_day => (theme::GOLD, theme::PEACH),
        Condition::Clear => (theme::LAVENDER, theme::CYAN),
        Condition::PartlyCloudy | Condition::Overcast => (theme::LAVENDER, theme::PALE),
        Condition::Fog => (theme::FOG_GREY, theme::PALE),
        Condition::Drizzle | Condition::Rain | Condition::RainShowers | Condition::Freezing => {
            (theme::CYAN, theme::LAVENDER)
        }
        Condition::Snow | Condition::SnowShowers => (theme::PALE, theme::CYAN),
        Condition::Thunderstorm => (theme::GOLD, theme::LAVENDER),
    }
}

pub fn icon_for(condition: Condition, is_day: bool) -> &'static str {
    match condition {
        Condition::Clear if is_day => "☀",
        Condition::Clear => "☾",
        Condition::PartlyCloudy => "⛅",
        Condition::Overcast => "☁",
        Condition::Fog => "▒",
        Condition::Drizzle => "⋰",
        Condition::Rain | Condition::RainShowers => "☂",
        Condition::Freezing => "☂",
        Condition::Snow | Condition::SnowShowers => "❄",
        Condition::Thunderstorm => "⚡",
    }
}
