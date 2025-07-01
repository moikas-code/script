mod scanner;
mod token;

pub use scanner::Lexer;
pub use token::{Token, TokenKind};

#[cfg(test)]
mod tests;