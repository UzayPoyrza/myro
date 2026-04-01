use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, AppState, OrderSortBy, PastFilter, PastOrder, SolveMode};
use crate::state::PastEntry;

impl App {
    pub(crate) fn handle_past_key(&mut self, key: KeyEvent) {
        if self.past_in_filter_popup() {
            self.handle_past_filter_key(key);
            return;
        }
        if self.past_in_order_popup() {
            self.handle_past_order_key(key);
            return;
        }
        if self.past_in_command_mode() {
            self.handle_past_command_input(key);
            return;
        }

        let filtered_len = self.filtered_past_entries().len();
        let current_scroll = match &self.state {
            AppState::Past { scroll, .. } => *scroll,
            _ => return,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let AppState::Past { scroll, .. } = &mut self.state {
                    if filtered_len > 0 {
                        *scroll = (current_scroll + 1).min(filtered_len.saturating_sub(1));
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let AppState::Past { scroll, .. } = &mut self.state {
                    *scroll = current_scroll.saturating_sub(1);
                }
            }
            KeyCode::Char('/') => self.open_past_command_input(),
            KeyCode::Enter => self.reopen_selected_past_problem(current_scroll),
            KeyCode::Esc | KeyCode::Char('q') => {
                self.state = AppState::Home { selected: 2 };
            }
            _ => {}
        }
    }

    fn past_in_filter_popup(&self) -> bool {
        matches!(&self.state, AppState::Past { filter_open: true, .. })
    }

    fn past_in_order_popup(&self) -> bool {
        matches!(&self.state, AppState::Past { order_open: true, .. })
    }

    fn past_in_command_mode(&self) -> bool {
        matches!(&self.state, AppState::Past { command_input: Some(_), .. })
    }

    fn open_past_command_input(&mut self) {
        if let AppState::Past { command_input, .. } = &mut self.state {
            *command_input = Some(String::new());
        }
    }

    fn open_past_filter_popup(&mut self) {
        if let AppState::Past {
            filter_open,
            filter_cursor,
            ..
        } = &mut self.state
        {
            *filter_open = true;
            *filter_cursor = 0;
        }
    }

    fn open_past_order_popup(&mut self) {
        if let AppState::Past {
            order_open,
            order_cursor,
            ..
        } = &mut self.state
        {
            *order_open = true;
            *order_cursor = 0;
        }
    }

    fn reopen_selected_past_problem(&mut self, current_scroll: usize) {
        let entries = self.filtered_past_entries();
        let Some(entry) = entries.get(current_scroll) else {
            return;
        };

        let contest_id = entry.contest_id;
        let index = entry.index.clone();
        let full_idx = self
            .past_entries
            .iter()
            .position(|pe| pe.contest_id == contest_id && pe.index == index);

        if let Some(idx) = full_idx {
            self.reopening_past_entry = Some(idx);
            self.last_solve_mode = Some(SolveMode::Chill);
            self.recommender
                .send(crate::recommend::RecommendRequest::FetchProblem {
                    contest_id,
                    index,
                });
            self.recommender.status = Some(format!(
                "fetching problem {}{}...",
                contest_id, self.past_entries[idx].index,
            ));
        }
    }

    fn handle_past_command_input(&mut self, key: KeyEvent) {
        let Some(cmd) = self.update_past_command_input(key) else {
            return;
        };
        self.execute_past_command(&cmd);
    }

    fn update_past_command_input(&mut self, key: KeyEvent) -> Option<String> {
        match &mut self.state {
            AppState::Past { command_input, .. } => {
                let cmd = command_input.as_mut()?;
                match key.code {
                    KeyCode::Esc => {
                        let _ = command_input.take();
                        None
                    }
                    KeyCode::Enter => {
                        let current = cmd.clone();
                        let _ = command_input.take();
                        Some(current)
                    }
                    KeyCode::Backspace => {
                        cmd.pop();
                        if cmd.is_empty() {
                            let _ = command_input.take();
                        }
                        None
                    }
                    KeyCode::Char(c) => {
                        cmd.push(c);
                        None
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn execute_past_command(&mut self, cmd: &str) {
        match cmd {
            "filter" => self.open_past_filter_popup(),
            "order" => self.open_past_order_popup(),
            _ => self.set_status(format!("unknown command: /{}", cmd)),
        }
    }

    fn handle_past_filter_key(&mut self, key: KeyEvent) {
        let cursor = match &mut self.state {
            AppState::Past {
                filter_cursor,
                filter_open: true,
                ..
            } => filter_cursor,
            _ => return,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                *cursor = (*cursor + 1).min(PastFilter::COUNT - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *cursor = cursor.saturating_sub(1);
            }
            KeyCode::Char(' ') => {
                self.past_filter.toggle(*cursor);
            }
            KeyCode::Esc => {
                if let AppState::Past {
                    filter_open, scroll, ..
                } = &mut self.state
                {
                    *filter_open = false;
                    *scroll = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_past_order_key(&mut self, key: KeyEvent) {
        let cursor = match &mut self.state {
            AppState::Past {
                order_cursor,
                order_open: true,
                ..
            } => order_cursor,
            _ => return,
        };

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                *cursor = (*cursor + 1).min(PastOrder::TOTAL_ITEMS - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                *cursor = cursor.saturating_sub(1);
            }
            KeyCode::Char(' ') => {
                if *cursor < 5 {
                    self.past_order.sort_by = OrderSortBy::ALL[*cursor];
                } else {
                    self.past_order.ascending = *cursor == 6;
                }
            }
            KeyCode::Esc => {
                if let AppState::Past {
                    order_open, scroll, ..
                } = &mut self.state
                {
                    *order_open = false;
                    *scroll = 0;
                }
            }
            _ => {}
        }
    }

    pub fn filtered_past_entries(&self) -> Vec<PastEntry> {
        let mut entries: Vec<PastEntry> = self
            .past_entries
            .iter()
            .filter(|entry| self.past_filter.matches(entry))
            .cloned()
            .collect();

        let sort_by = self.past_order.sort_by;
        let ascending = self.past_order.ascending;

        entries.sort_by(|a, b| {
            let cmp = match sort_by {
                OrderSortBy::FirstSeen => a.first_seen_at.cmp(&b.first_seen_at),
                OrderSortBy::LastSeen => a.last_seen_at.cmp(&b.last_seen_at),
                OrderSortBy::FirstSubmission => match (a.first_submitted_at, b.first_submitted_at)
                {
                    (Some(ta), Some(tb)) => ta.cmp(&tb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                },
                OrderSortBy::LastSubmission => match (a.last_submitted_at, b.last_submitted_at) {
                    (Some(ta), Some(tb)) => ta.cmp(&tb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                },
                OrderSortBy::Rating => match (a.rating, b.rating) {
                    (Some(ra), Some(rb)) => ra.cmp(&rb),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                },
            };
            if ascending { cmp } else { cmp.reverse() }
        });

        entries
    }
}
