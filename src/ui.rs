use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{palette::tailwind::SLATE, Color, Modifier, Style, Stylize},
    text::Text,
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, Paragraph},
};

use crate::app::App;

// Constants
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

pub fn render(frame: &mut ratatui::Frame<'_>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    // Create title and put into first chunk
    frame.render_widget(generate_header(), chunks[0]);
    frame.render_stateful_widget(generate_list(app), chunks[1], &mut app.confs.state);
    frame.render_widget(generate_footer(), chunks[2]);
}

fn generate_header() -> Paragraph<'static> {
    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled(
        "Your ssh configs",
        Style::default().fg(Color::Green),
    ))
    .block(title_block);
    title
}

fn generate_footer() -> Paragraph<'static> {
    let footer_block = Block::default()
        .borders(Borders::TOP)
        .style(Style::default());

    let footer =
        Paragraph::new("Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.")
            .centered()
            .block(footer_block);
    footer
}

fn generate_list(app: &App) -> List<'static> {
    let block = Block::new().style(Style::default());

    // Iterate through all elements in the `items` and stylize them.

    let items: Vec<ListItem> = app
        .confs
        .confs
        .iter()
        .enumerate()
        .map(|(i, host)| {
            let color = alternate_colors(i);
            ListItem::from(host).bg(color)
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let list = List::new(items)
        .block(block)
        .highlight_style(SELECTED_STYLE)
        .highlight_symbol(">")
        .highlight_spacing(HighlightSpacing::Always);

    list
}

const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}
