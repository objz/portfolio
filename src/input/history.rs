#[derive(Clone)]
pub struct CommandHistory {
    history: Vec<String>,
    current_index: Option<usize>,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_index: None,
        }
    }

    pub fn add(&mut self, command: String) {
        if !command.trim().is_empty() && self.history.last() != Some(&command) {
            self.history.push(command);
        }
        self.current_index = None;
    }

    pub fn prev(&mut self) -> Option<&String> {
        if self.history.is_empty() {
            return None;
        }

        let new_index = match self.current_index {
            None => self.history.len() - 1,
            Some(0) => return self.history.get(0),
            Some(i) => i - 1,
        };

        self.current_index = Some(new_index);
        self.history.get(new_index)
    }

    pub fn next(&mut self) -> Option<&String> {
        match self.current_index {
            None => None,
            Some(i) if i >= self.history.len() - 1 => {
                self.current_index = None;
                None
            }
            Some(i) => {
                let new_index = i + 1;
                self.current_index = Some(new_index);
                self.history.get(new_index)
            }
        }
    }
}
