use edtui::{EditorEventHandler, EditorState, Lines};

pub struct TestPanelState {
    pub input_state: EditorState,
    pub input_handler: EditorEventHandler,
    pub output_focused: bool,
    pub output_scroll: usize,
    pub run_progress: Option<(usize, usize)>,
    pub visible: bool,
}

impl TestPanelState {
    pub fn new() -> Self {
        Self {
            input_state: EditorState::new(Lines::from("")),
            input_handler: EditorEventHandler::default(),
            output_focused: false,
            output_scroll: 0,
            run_progress: None,
            visible: true,
        }
    }

    pub fn get_input(&self) -> String {
        self.input_state.lines.to_string()
    }

    pub fn set_input(&mut self, text: &str) {
        self.input_state = EditorState::new(Lines::from(text));
        self.input_handler = EditorEventHandler::default();
    }
}
