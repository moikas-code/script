# Script Documentation Generator

The Script language includes a built-in documentation generator that produces clean, searchable HTML documentation from your code comments.

## Features

- **Doc Comment Support**: Parse `///` and `/** */` style documentation comments
- **Structured Documentation**: Support for `@param`, `@returns`, `@example`, `@note`, and `@see` tags
- **HTML Generation**: Clean, responsive HTML output with syntax highlighting
- **Search Functionality**: Built-in JavaScript search across all documented items
- **Module Organization**: Hierarchical documentation structure matching your code organization

## Usage

### Writing Documentation Comments

```script
/// Calculate the area of a rectangle
/// @param width - The width of the rectangle
/// @param height - The height of the rectangle  
/// @returns The area as width * height
fn rectangle_area(width, height) {
    return width * height
}
```

### Generating Documentation

```bash
# Generate docs for a directory
script doc ./src ./docs

# Generate docs with default output directory (./docs)
script doc ./src
```

## Documentation Tags

- `@param name [type] - description`: Document function parameters
- `@returns description`: Document return values
- `@example [title]`: Add code examples (use ``` for code blocks)
- `@note text`: Add important notes
- `@see reference`: Add cross-references

## Architecture

The documentation generator consists of:

1. **Lexer Integration**: Extended to recognize doc comment tokens
2. **Documentation Parser**: Extracts structured information from comments
3. **HTML Generator**: Produces static HTML with embedded CSS/JS
4. **Search Engine**: Builds and uses an index for fast searching