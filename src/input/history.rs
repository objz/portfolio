#[derive(Clone)]
pub struct CommandHistory {
    history: Vec<String>,
    current_index: Option<usize>,
    draft_input: Option<String>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_index: None,
            draft_input: None,
        }
    }

    pub fn add(&mut self, command: String) {
        if !command.trim().is_empty() && self.history.last() != Some(&command) {
            self.history.push(command);
        }
        self.reset_navigation();
    }

    pub fn prev(&mut self, current_input: &str) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            None => {
                self.draft_input = Some(current_input.to_string());
                self.history.len() - 1
            }
            Some(0) => 0,
            Some(i) => i - 1,
        };

        self.current_index = Some(new_index);
        self.history.get(new_index).cloned()
    }

    pub fn next(&mut self) -> Option<String> {
        match self.current_index {
            None => None,
            Some(i) if i >= self.history.len() - 1 => {
                self.current_index = None;
                self.draft_input.take()
            }
            Some(i) => {
                let new_index = i + 1;
                self.current_index = Some(new_index);
                self.history.get(new_index).cloned()
            }
        }
    }

    pub fn reset_navigation(&mut self) {
        self.current_index = None;
        self.draft_input = None;
    }

    pub fn suggest(&self, prefix: &str) -> Option<String> {
        let normalized = prefix.trim();
        if normalized.is_empty() {
            return None;
        }

        self.history
            .iter()
            .rev()
            .find(|entry| entry.starts_with(normalized) && entry.as_str() != normalized)
            .cloned()
    }
}
