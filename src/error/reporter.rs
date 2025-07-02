use super::Error;
use colored::*;

pub struct ErrorReporter {
    errors: Vec<Error>,
}

impl ErrorReporter {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn report(&mut self, error: Error) {
        self.errors.push(error);
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    pub fn print_all(&self) {
        for error in &self.errors {
            eprintln!("{}\n", error);
        }

        if !self.errors.is_empty() {
            let count = self.errors.len();
            let plural = if count == 1 { "" } else { "s" };
            eprintln!(
                "{} {} error{} generated",
                "Script:".cyan().bold(),
                count.to_string().red().bold(),
                plural
            );
        }
    }
}

impl Default for ErrorReporter {
    fn default() -> Self {
        Self::new()
    }
}
