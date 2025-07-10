mod lru_cache;
mod scanner;
mod token;

#[cfg(feature = "fuzzing")]
pub mod fuzz;

pub use scanner::{Lexer, UnicodeSecurityConfig, UnicodeSecurityLevel};
pub use token::{Token, TokenKind};

#[cfg(test)]
mod tests;
