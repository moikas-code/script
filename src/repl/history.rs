use std::collections::VecDeque;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

/// Maximum number of history entries to keep
const MAX_HISTORY_SIZE: usize = 1000;

/// Command history manager with persistent storage
pub struct History {
    /// Command history storage
    commands: VecDeque<String>,
    /// Path to history file
    history_file: PathBuf,
    /// Maximum number of entries to keep
    max_size: usize,
}

impl History {
    /// Create a new history manager
    pub fn new(history_file: PathBuf) -> Self {
        History {
            commands: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            history_file,
            max_size: MAX_HISTORY_SIZE,
        }
    }

    /// Load existing history or create new one
    pub fn load_or_create() -> io::Result<Self> {
        let history_file = Self::get_history_file_path()?;
        let mut history = Self::new(history_file.clone());

        if history_file.exists() {
            history.load_from_file()?;
        }

        Ok(history)
    }

    /// Get the default history file path
    fn get_history_file_path() -> io::Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;

        Ok(home_dir.join(".script_history"))
    }

    /// Load history from file
    fn load_from_file(&mut self) -> io::Result<()> {
        let file = fs::File::open(&self.history_file)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                self.commands.push_back(line);
            }
        }

        // Trim to max size if needed
        while self.commands.len() > self.max_size {
            self.commands.pop_front();
        }

        Ok(())
    }

    /// Save history to file
    pub fn save(&self) -> io::Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.history_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(&self.history_file)?;

        for command in &self.commands {
            writeln!(file, "{}", command)?;
        }

        Ok(())
    }

    /// Add a command to history
    pub fn add(&mut self, command: String) {
        // Don't add empty commands or duplicates of the last command
        if command.trim().is_empty() {
            return;
        }

        if let Some(last) = self.commands.back() {
            if last == &command {
                return;
            }
        }

        self.commands.push_back(command);

        // Maintain max size
        if self.commands.len() > self.max_size {
            self.commands.pop_front();
        }
    }

    /// Get recent commands (up to specified count)
    pub fn recent(&self, count: usize) -> Vec<&String> {
        self.commands
            .iter()
            .rev()
            .take(count)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get all commands
    pub fn all(&self) -> &VecDeque<String> {
        &self.commands
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Get the number of commands in history
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Search history for commands containing the given text
    pub fn search(&self, text: &str) -> Vec<&String> {
        self.commands
            .iter()
            .filter(|cmd| cmd.contains(text))
            .collect()
    }

    /// Get command by index (0 is oldest)
    pub fn get(&self, index: usize) -> Option<&String> {
        self.commands.get(index)
    }

    /// Remove command at index
    pub fn remove(&mut self, index: usize) -> Option<String> {
        self.commands.remove(index)
    }
}

impl Default for History {
    fn default() -> Self {
        Self::load_or_create().unwrap_or_else(|_| {
            // Fallback to in-memory only if file operations fail
            Self::new(PathBuf::from(".script_history"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_history_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        // Test adding commands
        history.add("let x = 5".to_string());
        history.add("let y = 10".to_string());
        history.add("x + y".to_string());

        assert_eq!(history.len(), 3);
        assert!(!history.is_empty());

        // Test recent commands
        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], "let y = 10");
        assert_eq!(recent[1], "x + y");

        // Test search
        let matches = history.search("let");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_history_persistence() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");

        // Create and populate history
        {
            let mut history = History::new(history_file.clone());
            history.add("command 1".to_string());
            history.add("command 2".to_string());
            history.save().unwrap();
        }

        // Load history and verify
        {
            let mut history = History::new(history_file);
            history.load_from_file().unwrap();
            assert_eq!(history.len(), 2);
            assert_eq!(history.get(0).unwrap(), "command 1");
            assert_eq!(history.get(1).unwrap(), "command 2");
        }
    }

    #[test]
    fn test_history_deduplication() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);

        history.add("same command".to_string());
        history.add("same command".to_string()); // Should be ignored
        history.add("different command".to_string());

        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_history_max_size() {
        let temp_dir = tempdir().unwrap();
        let history_file = temp_dir.path().join("test_history");
        let mut history = History::new(history_file);
        history.max_size = 3; // Set small max size for testing

        // Add more commands than max size
        for i in 0..5 {
            history.add(format!("command {}", i));
        }

        assert_eq!(history.len(), 3);
        // Should have the last 3 commands
        assert_eq!(history.get(0).unwrap(), "command 2");
        assert_eq!(history.get(2).unwrap(), "command 4");
    }
}
