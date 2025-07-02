pub mod generator;
pub mod html;
pub mod search;

#[cfg(test)]
mod tests;

use crate::source::Span;
use std::collections::HashMap;

/// Documentation information for items in the Script language
#[derive(Debug, Clone)]
pub struct Documentation {
    /// The raw documentation content
    pub content: String,
    /// Parsed sections (description, parameters, returns, etc.)
    pub sections: DocSections,
    /// Source location of the documentation
    pub span: Span,
}

/// Parsed documentation sections
#[derive(Debug, Clone, Default)]
pub struct DocSections {
    /// Main description
    pub description: String,
    /// Parameter documentation
    pub params: Vec<ParamDoc>,
    /// Return value documentation
    pub returns: Option<String>,
    /// Example code blocks
    pub examples: Vec<Example>,
    /// Additional notes
    pub notes: Vec<String>,
    /// See also references
    pub see_also: Vec<String>,
}

/// Documentation for a function parameter
#[derive(Debug, Clone)]
pub struct ParamDoc {
    pub name: String,
    pub type_info: Option<String>,
    pub description: String,
}

/// Example code block
#[derive(Debug, Clone)]
pub struct Example {
    pub title: Option<String>,
    pub code: String,
    pub output: Option<String>,
}

/// Module documentation information
#[derive(Debug, Clone)]
pub struct ModuleDoc {
    /// Module name
    pub name: String,
    /// Module path (e.g., "std::io")
    pub path: String,
    /// Module-level documentation
    pub documentation: Option<Documentation>,
    /// Functions in this module
    pub functions: Vec<FunctionDoc>,
    /// Types defined in this module
    pub types: Vec<TypeDoc>,
    /// Constants in this module
    pub constants: Vec<ConstantDoc>,
    /// Sub-modules
    pub submodules: Vec<String>,
}

/// Function documentation
#[derive(Debug, Clone)]
pub struct FunctionDoc {
    pub name: String,
    pub signature: String,
    pub documentation: Option<Documentation>,
    pub is_async: bool,
    pub is_exported: bool,
}

/// Type documentation
#[derive(Debug, Clone)]
pub struct TypeDoc {
    pub name: String,
    pub kind: TypeKind,
    pub documentation: Option<Documentation>,
    pub methods: Vec<FunctionDoc>,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct,
    Enum,
    Interface,
    TypeAlias,
}

/// Constant documentation
#[derive(Debug, Clone)]
pub struct ConstantDoc {
    pub name: String,
    pub type_info: String,
    pub value: Option<String>,
    pub documentation: Option<Documentation>,
}

/// Parse raw documentation comments into structured documentation
pub fn parse_doc_comments(comments: Vec<String>) -> Documentation {
    let content = comments.join("\n");
    let sections = parse_sections(&content);

    Documentation {
        content,
        sections,
        span: Span::dummy(), // Will be updated with actual span
    }
}

/// Parse documentation content into sections
fn parse_sections(content: &str) -> DocSections {
    let mut sections = DocSections::default();
    let mut current_section = String::new();
    let mut in_example = false;
    let mut current_example = None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for special tags
        if trimmed.starts_with("@param") || trimmed.starts_with("@arg") {
            // Parse parameter documentation
            if let Some(param) = parse_param_doc(trimmed) {
                sections.params.push(param);
            }
        } else if trimmed.starts_with("@returns") || trimmed.starts_with("@return") {
            // Parse return documentation
            let return_doc = trimmed
                .trim_start_matches("@returns")
                .trim_start_matches("@return")
                .trim();
            sections.returns = Some(return_doc.to_string());
        } else if trimmed.starts_with("@example") {
            // Start new example
            in_example = true;
            let title = trimmed.trim_start_matches("@example").trim();
            current_example = Some(Example {
                title: if title.is_empty() {
                    None
                } else {
                    Some(title.to_string())
                },
                code: String::new(),
                output: None,
            });
        } else if trimmed.starts_with("@note") {
            // Add note
            let note = trimmed.trim_start_matches("@note").trim();
            sections.notes.push(note.to_string());
        } else if trimmed.starts_with("@see") {
            // Add see also reference
            let reference = trimmed.trim_start_matches("@see").trim();
            sections.see_also.push(reference.to_string());
        } else if in_example && trimmed.starts_with("```") {
            // Toggle code block in example
            if let Some(ref mut example) = current_example {
                if example.code.is_empty() {
                    // Starting code block
                    continue;
                } else {
                    // Ending code block
                    sections.examples.push(current_example.take().unwrap());
                    in_example = false;
                }
            }
        } else if in_example {
            // Add to current example
            if let Some(ref mut example) = current_example {
                example.code.push_str(line);
                example.code.push('\n');
            }
        } else {
            // Regular description text
            if !current_section.is_empty() {
                current_section.push('\n');
            }
            current_section.push_str(trimmed);
        }
    }

    // Handle any remaining example
    if let Some(example) = current_example {
        sections.examples.push(example);
    }

    sections.description = current_section.trim().to_string();
    sections
}

/// Parse a parameter documentation line
fn parse_param_doc(line: &str) -> Option<ParamDoc> {
    let content = line
        .trim_start_matches("@param")
        .trim_start_matches("@arg")
        .trim();

    // Try to parse: name [type] - description
    let parts: Vec<&str> = content.splitn(2, '-').collect();
    if parts.is_empty() {
        return None;
    }

    let name_and_type = parts[0].trim();
    let description = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();

    // Check if type is specified in brackets
    if let Some(start) = name_and_type.find('[') {
        if let Some(end) = name_and_type.find(']') {
            let name = name_and_type[..start].trim().to_string();
            let type_info = name_and_type[start + 1..end].trim().to_string();
            return Some(ParamDoc {
                name,
                type_info: Some(type_info),
                description,
            });
        }
    }

    // No type specified
    Some(ParamDoc {
        name: name_and_type.to_string(),
        type_info: None,
        description,
    })
}

/// Documentation database for storing all documented items
#[derive(Debug, Default)]
pub struct DocDatabase {
    /// All modules in the documentation
    pub modules: HashMap<String, ModuleDoc>,
    /// Global search index
    pub search_index: SearchIndex,
}

/// Search index for documentation
#[derive(Debug, Default)]
pub struct SearchIndex {
    /// Map from search terms to item paths
    pub terms: HashMap<String, Vec<SearchResult>>,
}

/// A search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub name: String,
    pub kind: ItemKind,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Module,
    Function,
    Type,
    Constant,
    Method,
}
