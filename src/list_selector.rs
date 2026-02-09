use anyhow::Result;
use crossterm::event::{self, KeyCode};
use log::{error, warn};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListState};
use ratatui::Frame;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::exit;

pub trait ListSelector {
    fn select_one(&self, files: BTreeMap<String, PathBuf>) -> Option<PathBuf>;
}

pub struct TuiListSelector {}

impl TuiListSelector {
    pub fn new() -> TuiListSelector {
        TuiListSelector {}
    }
}

impl ListSelector for TuiListSelector {
    fn select_one(&self, files: BTreeMap<String, PathBuf>) -> Option<PathBuf> {
        if files.is_empty() {
            warn!("Could not find any file");
            return None;
        }

        if files.len() == 1 {
            return Some(files.values().next().unwrap().clone());
        }

        Self::tui(&files).unwrap_or_else(|e| {
            error!("Error in Terminal UI: {}", e);
            exit(1);
        })
    }
}

impl TuiListSelector {
    pub fn tui(files: &BTreeMap<String, PathBuf>) -> Result<Option<PathBuf>> {
        let mut list_state = ListState::default().with_selected(Some(0));
        let mut selected: Option<usize> = None;
        ratatui::run(|terminal| loop {
            terminal.draw(|frame| {
                Self::render(frame, &mut list_state, files.keys().cloned().collect())
            })?;
            if let Some(key) = event::read()?.as_key_press_event() {
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => list_state.select_next(),
                    KeyCode::Char('k') | KeyCode::Up => list_state.select_previous(),
                    KeyCode::Enter => {
                        selected = list_state.selected();
                        break Ok(selected);
                    }
                    KeyCode::Char('q') | KeyCode::Esc => break Ok(None),
                    _ => {}
                }
            }
        })
        .map(|s| match s {
            Some(i) => Some(files.values().nth(i).unwrap().clone()),
            None => None,
        })
    }

    fn render(frame: &mut Frame, list_state: &mut ListState, items: Vec<String>) {
        let constraints = [Constraint::Length(1), Constraint::Fill(1)];
        let layout = Layout::vertical(constraints).spacing(1);
        let [top, first] = frame.area().layout(&layout);

        let title = Line::from_iter([
            Span::from("Select file").bold(),
            Span::from(" (Press 'q' to quit and arrow keys to navigate)"),
        ]);
        frame.render_widget(title.centered(), top);

        let list = List::new(items)
            .style(Color::Gray)
            .highlight_style(Modifier::REVERSED);

        frame.render_stateful_widget(list, first, list_state);
    }
}
