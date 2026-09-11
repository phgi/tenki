use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{Search, SearchStatus};
use crate::net::geocode::Place;
use crate::ui::detail::centered_rect;
use crate::ui::theme::{self, rgb};

const PANEL_WIDTH: u16 = 56;
/// Border, prompt row, status row, border.
const CHROME_HEIGHT: u16 = 4;
/// The "▸ " selection marker, indented one column to match the prompt.
const MARKER_WIDTH: usize = 3;

pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    match max {
        0 => String::new(),
        1 => "…".to_string(),
        _ => text.chars().take(max - 1).chain(['…']).collect(),
    }
}

fn prompt_line(query: &str, inner: usize) -> Line<'static> {
    // In a popup this narrow the chrome alone fills the row.
    if inner < 5 {
        return Line::default();
    }
    // The tail of a long query matters more than its head while typing.
    let room = inner.saturating_sub(5);
    let shown: String = if query.chars().count() > room {
        query.chars().skip(query.chars().count() - room).collect()
    } else {
        query.to_string()
    };
    Line::from(vec![
        Span::styled("  > ", Style::default().fg(rgb(theme::PINK))),
        Span::styled(
            shown,
            Style::default()
                .fg(rgb(theme::PALE))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("▌", Style::default().fg(rgb(theme::CYAN))),
    ])
}

fn status_line(search: &Search, inner: usize) -> Line<'static> {
    let (text, color) = match &search.status {
        SearchStatus::Typing if search.results.is_empty() => (
            "  type a city, a postcode, or \"lat, lon\"".to_string(),
            theme::MUTED_TEXT,
        ),
        SearchStatus::Typing => (format!("  {} matches", search.results.len()), theme::MUTED_TEXT),
        SearchStatus::Searching => ("  searching…".to_string(), theme::CYAN),
        SearchStatus::NoMatches => (
            format!("  nothing found for \"{}\"", search.submitted),
            theme::PEACH,
        ),
        SearchStatus::Failed(e) => (format!("  {e}"), theme::PEACH),
    };
    Line::from(Span::styled(
        truncate(&text, inner),
        Style::default().fg(rgb(color)),
    ))
}

fn result_line(place: &Place, selected: bool, inner: usize) -> Line<'static> {
    // No room for the marker and a letter of the name: draw nothing at all.
    if inner < 5 {
        return Line::default();
    }
    let coords = place.coordinates();
    // Below ~34 columns the coordinate column costs more than it tells.
    let show_coords = inner >= coords.chars().count() + 20;
    // Marker, then the label, then a gap, the coordinates and a right-hand margin.
    let label_room = if show_coords {
        inner - coords.chars().count() - MARKER_WIDTH - 2
    } else {
        inner - MARKER_WIDTH - 1
    };
    let label = truncate(&place.label, label_room);

    let (marker, label_style) = if selected {
        (
            " ▸ ",
            Style::default()
                .fg(rgb(theme::PALE))
                .add_modifier(Modifier::BOLD),
        )
    } else {
        ("   ", Style::default().fg(rgb(theme::MUTED_TEXT)))
    };

    let mut spans = vec![
        Span::styled(marker, Style::default().fg(rgb(theme::PINK))),
        Span::styled(label.clone(), label_style),
    ];
    if show_coords {
        let used = MARKER_WIDTH + label.chars().count() + coords.chars().count();
        spans.push(Span::raw(" ".repeat(inner.saturating_sub(used + 1))));
        spans.push(Span::styled(
            coords,
            Style::default().fg(rgb(theme::LAVENDER)),
        ));
    }
    Line::from(spans)
}

fn hint(search: &Search, width: u16) -> String {
    let narrow = width < 44;
    if search.results.is_empty() {
        if narrow {
            " ⏎ search · esc ".to_string()
        } else {
            " ⏎ search · esc cancel ".to_string()
        }
    } else if narrow {
        " ↑↓ · ⏎ use · esc ".to_string()
    } else {
        " ↑↓ select · ⏎ use · esc cancel ".to_string()
    }
}

pub fn render(frame: &mut Frame, search: &Search) {
    let rows = search.results.len() as u16;
    let popup = centered_rect(PANEL_WIDTH, CHROME_HEIGHT + rows, frame.area());
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(rgb(theme::LAVENDER)))
        .style(Style::default().bg(rgb(theme::BG_DIM)))
        .title(Span::styled(
            " where are you? ",
            Style::default()
                .fg(rgb(theme::PINK))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .title_bottom(
            Line::from(Span::styled(
                hint(search, popup.width),
                Style::default().fg(rgb(theme::MUTED_TEXT)),
            ))
            .centered(),
        );

    let inner = popup.width.saturating_sub(2) as usize;
    let mut lines = vec![prompt_line(&search.query, inner), status_line(search, inner)];
    for (i, place) in search.results.iter().enumerate() {
        lines.push(result_line(place, i == search.selected, inner));
    }

    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_leaves_short_text_alone() {
        assert_eq!(truncate("Berlin", 10), "Berlin");
        assert_eq!(truncate("Berlin", 6), "Berlin");
    }

    #[test]
    fn truncate_marks_what_it_cut() {
        assert_eq!(truncate("Berlin", 5), "Berl…");
        assert_eq!(truncate("Berlin", 1), "…");
        assert_eq!(truncate("Berlin", 0), "");
    }

    #[test]
    fn a_result_row_never_exceeds_the_inner_width() {
        let place = Place::from_coordinates(52.5200, 13.4048);
        for inner in 0..60usize {
            let width: usize = result_line(&place, true, inner)
                .spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum();
            assert!(width <= inner, "row of {width} overflows inner width {inner}");
        }
    }

    #[test]
    fn a_long_label_is_truncated_rather_than_wrapped() {
        let mut place = Place::from_coordinates(1.0, 2.0);
        place.label = "A very long district name, In Some Region, In Some Country".into();
        let row: String = result_line(&place, false, 40)
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(row.chars().count() <= 40, "row too wide: {row:?}");
        assert!(row.contains('…'), "no truncation marker: {row:?}");
    }

    #[test]
    fn the_prompt_never_exceeds_the_inner_width() {
        for inner in 0..60usize {
            let width: usize = prompt_line("berlin mitte", inner)
                .spans
                .iter()
                .map(|s| s.content.chars().count())
                .sum();
            assert!(width <= inner, "prompt of {width} overflows inner width {inner}");
        }
    }

    #[test]
    fn the_prompt_scrolls_to_keep_the_caret_visible() {
        let long = "a".repeat(200);
        let line = prompt_line(&long, 30);
        let width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        assert!(width <= 30, "prompt of {width} overflows 30");
        assert!(line.spans.last().unwrap().content.contains('▌'));
    }
}
