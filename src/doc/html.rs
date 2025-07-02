use super::*;
use std::fs;
use std::path::{Path, PathBuf};

/// HTML documentation generator
pub struct HtmlGenerator {
    /// Output directory
    output_dir: PathBuf,
    /// CSS styles
    css: String,
    /// JavaScript for search functionality
    js: String,
}

impl HtmlGenerator {
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            css: Self::default_css(),
            js: Self::default_js(),
        }
    }

    /// Generate HTML documentation from a documentation database
    pub fn generate(&self, database: &DocDatabase) -> std::io::Result<()> {
        // Create output directory
        fs::create_dir_all(&self.output_dir)?;

        // Write static files
        self.write_static_files()?;

        // Generate index page
        self.generate_index(database)?;

        // Generate module pages
        for (path, module) in &database.modules {
            self.generate_module_page(path, module)?;
        }

        // Generate search index
        self.generate_search_index(database)?;

        Ok(())
    }

    /// Write static CSS and JS files
    fn write_static_files(&self) -> std::io::Result<()> {
        let css_path = self.output_dir.join("style.css");
        fs::write(css_path, &self.css)?;

        let js_path = self.output_dir.join("search.js");
        fs::write(js_path, &self.js)?;

        Ok(())
    }

    /// Generate the index page
    fn generate_index(&self, database: &DocDatabase) -> std::io::Result<()> {
        let mut html = String::new();

        html.push_str(&self.html_header("Script Documentation", ""));

        html.push_str(
            r#"
        <div class="container">
            <h1>Script Language Documentation</h1>
            
            <div class="search-container">
                <input type="text" id="search-input" placeholder="Search documentation..." />
                <div id="search-results"></div>
            </div>
            
            <h2>Modules</h2>
            <div class="module-list">
        "#,
        );

        // List all modules
        let mut modules: Vec<_> = database.modules.keys().collect();
        modules.sort();

        for module_name in modules {
            let module = &database.modules[module_name];
            let doc_summary = module
                .documentation
                .as_ref()
                .map(|d| d.sections.description.as_str())
                .unwrap_or("");

            html.push_str(&format!(
                r#"
                <div class="module-item">
                    <h3><a href="{}.html">{}</a></h3>
                    <p>{}</p>
                </div>
            "#,
                self.module_filename(module_name),
                module_name,
                self.escape_html(&self.first_line(doc_summary))
            ));
        }

        html.push_str(
            r#"
            </div>
        </div>
        "#,
        );

        html.push_str(&self.html_footer());

        let index_path = self.output_dir.join("index.html");
        fs::write(index_path, html)?;

        Ok(())
    }

    /// Generate a module documentation page
    fn generate_module_page(&self, path: &str, module: &ModuleDoc) -> std::io::Result<()> {
        let mut html = String::new();

        html.push_str(&self.html_header(&format!("{} - Script Documentation", module.name), "../"));

        html.push_str(&format!(
            r#"
        <div class="container">
            <nav class="breadcrumb">
                <a href="index.html">Script Documentation</a> &gt; {}
            </nav>
            
            <h1>Module {}</h1>
        "#,
            module.name, module.name
        ));

        // Module documentation
        if let Some(doc) = &module.documentation {
            html.push_str(&self.render_documentation(doc));
        }

        // Table of contents
        html.push_str(r#"<div class="toc"><h2>Contents</h2><ul>"#);

        if !module.functions.is_empty() {
            html.push_str(&format!(
                r##"<li><a href="#functions">Functions ({})</a></li>"##,
                module.functions.len()
            ));
        }
        if !module.types.is_empty() {
            html.push_str(&format!(
                r##"<li><a href="#types">Types ({})</a></li>"##,
                module.types.len()
            ));
        }
        if !module.constants.is_empty() {
            html.push_str(&format!(
                r##"<li><a href="#constants">Constants ({})</a></li>"##,
                module.constants.len()
            ));
        }

        html.push_str("</ul></div>");

        // Functions
        if !module.functions.is_empty() {
            html.push_str(r#"<section id="functions"><h2>Functions</h2>"#);

            for func in &module.functions {
                html.push_str(&self.render_function(func));
            }

            html.push_str("</section>");
        }

        // Types
        if !module.types.is_empty() {
            html.push_str(r#"<section id="types"><h2>Types</h2>"#);

            for type_doc in &module.types {
                html.push_str(&self.render_type(type_doc));
            }

            html.push_str("</section>");
        }

        // Constants
        if !module.constants.is_empty() {
            html.push_str(r#"<section id="constants"><h2>Constants</h2>"#);

            for const_doc in &module.constants {
                html.push_str(&self.render_constant(const_doc));
            }

            html.push_str("</section>");
        }

        html.push_str("</div>");
        html.push_str(&self.html_footer());

        let file_path = self
            .output_dir
            .join(format!("{}.html", self.module_filename(path)));
        fs::write(file_path, html)?;

        Ok(())
    }

    /// Render a function documentation
    fn render_function(&self, func: &FunctionDoc) -> String {
        let mut html = String::new();

        html.push_str(&format!(
            r#"
        <div class="item function" id="fn.{}">
            <h3>{}</h3>
            <pre class="signature"><code>{}</code></pre>
        "#,
            func.name,
            func.name,
            self.escape_html(&func.signature)
        ));

        if let Some(doc) = &func.documentation {
            html.push_str(&self.render_documentation(doc));
        }

        html.push_str("</div>");

        html
    }

    /// Render a type documentation
    fn render_type(&self, type_doc: &TypeDoc) -> String {
        let mut html = String::new();

        let type_kind = match type_doc.kind {
            TypeKind::Struct => "struct",
            TypeKind::Enum => "enum",
            TypeKind::Interface => "interface",
            TypeKind::TypeAlias => "type",
        };

        html.push_str(&format!(
            r#"
        <div class="item type" id="type.{}">
            <h3>{} {}</h3>
        "#,
            type_doc.name, type_kind, type_doc.name
        ));

        if let Some(doc) = &type_doc.documentation {
            html.push_str(&self.render_documentation(doc));
        }

        // Methods
        if !type_doc.methods.is_empty() {
            html.push_str(r#"<div class="methods"><h4>Methods</h4>"#);
            for method in &type_doc.methods {
                html.push_str(&self.render_function(method));
            }
            html.push_str("</div>");
        }

        html.push_str("</div>");

        html
    }

    /// Render a constant documentation
    fn render_constant(&self, const_doc: &ConstantDoc) -> String {
        let mut html = String::new();

        html.push_str(&format!(
            r#"
        <div class="item constant" id="const.{}">
            <h3>{}</h3>
            <pre class="signature"><code>const {}: {}"#,
            const_doc.name,
            const_doc.name,
            const_doc.name,
            self.escape_html(&const_doc.type_info)
        ));

        if let Some(value) = &const_doc.value {
            html.push_str(&format!(" = {}", self.escape_html(value)));
        }

        html.push_str("</code></pre>");

        if let Some(doc) = &const_doc.documentation {
            html.push_str(&self.render_documentation(doc));
        }

        html.push_str("</div>");

        html
    }

    /// Render documentation sections
    fn render_documentation(&self, doc: &Documentation) -> String {
        let mut html = String::new();

        html.push_str(r#"<div class="documentation">"#);

        // Description
        if !doc.sections.description.is_empty() {
            html.push_str(&format!(
                "<p>{}</p>",
                self.escape_html(&doc.sections.description)
            ));
        }

        // Parameters
        if !doc.sections.params.is_empty() {
            html.push_str(r#"<div class="params"><h4>Parameters</h4><ul>"#);

            for param in &doc.sections.params {
                html.push_str(&format!(
                    r#"<li><code>{}</code>"#,
                    self.escape_html(&param.name)
                ));

                if let Some(type_info) = &param.type_info {
                    html.push_str(&format!(
                        r#" : <span class="type">{}</span>"#,
                        self.escape_html(type_info)
                    ));
                }

                if !param.description.is_empty() {
                    html.push_str(&format!(" - {}", self.escape_html(&param.description)));
                }

                html.push_str("</li>");
            }

            html.push_str("</ul></div>");
        }

        // Returns
        if let Some(returns) = &doc.sections.returns {
            html.push_str(&format!(
                r#"<div class="returns"><h4>Returns</h4><p>{}</p></div>"#,
                self.escape_html(returns)
            ));
        }

        // Examples
        if !doc.sections.examples.is_empty() {
            html.push_str(r#"<div class="examples"><h4>Examples</h4>"#);

            for example in &doc.sections.examples {
                if let Some(title) = &example.title {
                    html.push_str(&format!("<h5>{}</h5>", self.escape_html(title)));
                }

                html.push_str(&format!(
                    r#"<pre><code class="language-script">{}</code></pre>"#,
                    self.escape_html(&example.code)
                ));

                if let Some(output) = &example.output {
                    html.push_str(&format!(
                        r#"<pre class="output"><code>{}</code></pre>"#,
                        self.escape_html(output)
                    ));
                }
            }

            html.push_str("</div>");
        }

        // Notes
        if !doc.sections.notes.is_empty() {
            html.push_str(r#"<div class="notes"><h4>Notes</h4><ul>"#);

            for note in &doc.sections.notes {
                html.push_str(&format!("<li>{}</li>", self.escape_html(note)));
            }

            html.push_str("</ul></div>");
        }

        // See also
        if !doc.sections.see_also.is_empty() {
            html.push_str(r#"<div class="see-also"><h4>See Also</h4><ul>"#);

            for reference in &doc.sections.see_also {
                html.push_str(&format!("<li>{}</li>", self.escape_html(reference)));
            }

            html.push_str("</ul></div>");
        }

        html.push_str("</div>");

        html
    }

    /// Generate the search index JSON file
    fn generate_search_index(&self, database: &DocDatabase) -> std::io::Result<()> {
        let mut index_data = String::from("var SEARCH_INDEX = {\n");

        for (term, results) in &database.search_index.terms {
            index_data.push_str(&format!("  \"{}\": [\n", self.escape_js(term)));

            for result in results {
                index_data.push_str(&format!(
                    "    {{ path: \"{}\", name: \"{}\", kind: \"{:?}\", summary: \"{}\" }},\n",
                    self.escape_js(&result.path),
                    self.escape_js(&result.name),
                    result.kind,
                    self.escape_js(&result.summary)
                ));
            }

            index_data.push_str("  ],\n");
        }

        index_data.push_str("};\n");

        let index_path = self.output_dir.join("search-index.js");
        fs::write(index_path, index_data)?;

        Ok(())
    }

    /// HTML header template
    fn html_header(&self, title: &str, path_prefix: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="{}style.css">
    <script src="{}search-index.js"></script>
    <script src="{}search.js"></script>
</head>
<body>
"#,
            title, path_prefix, path_prefix, path_prefix
        )
    }

    /// HTML footer template
    fn html_footer(&self) -> &'static str {
        r#"
</body>
</html>"#
    }

    /// Get the filename for a module
    fn module_filename(&self, module_path: &str) -> String {
        module_path.replace("::", "_")
    }

    /// Escape HTML special characters
    fn escape_html(&self, text: &str) -> String {
        text.replace("&", "&amp;")
            .replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("\"", "&quot;")
            .replace("'", "&#39;")
    }

    /// Escape JavaScript string
    fn escape_js(&self, text: &str) -> String {
        text.replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
    }

    /// Get the first line of text
    fn first_line<'a>(&self, text: &'a str) -> &'a str {
        text.lines().next().unwrap_or("")
    }

    /// Default CSS styles
    fn default_css() -> String {
        r#"
/* Reset and base styles */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    line-height: 1.6;
    color: #333;
    background-color: #fff;
}

.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
}

/* Typography */
h1, h2, h3, h4, h5, h6 {
    margin-top: 1.5em;
    margin-bottom: 0.5em;
    color: #222;
}

h1 { font-size: 2.5em; }
h2 { font-size: 2em; }
h3 { font-size: 1.5em; }
h4 { font-size: 1.2em; }

a {
    color: #0066cc;
    text-decoration: none;
}

a:hover {
    text-decoration: underline;
}

code {
    font-family: "SF Mono", Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Courier New", monospace;
    background-color: #f5f5f5;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 0.9em;
}

pre {
    background-color: #f5f5f5;
    padding: 15px;
    border-radius: 5px;
    overflow-x: auto;
    margin: 1em 0;
}

pre code {
    background-color: transparent;
    padding: 0;
}

/* Navigation */
.breadcrumb {
    padding: 10px 0;
    color: #666;
}

.breadcrumb a {
    color: #666;
}

/* Search */
.search-container {
    margin: 20px 0;
}

#search-input {
    width: 100%;
    padding: 10px;
    font-size: 16px;
    border: 1px solid #ddd;
    border-radius: 5px;
}

#search-results {
    margin-top: 10px;
    padding: 10px;
    background-color: #f9f9f9;
    border-radius: 5px;
    display: none;
}

#search-results.active {
    display: block;
}

.search-result {
    padding: 5px 0;
    border-bottom: 1px solid #eee;
}

.search-result:last-child {
    border-bottom: none;
}

/* Module list */
.module-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 20px;
    margin-top: 20px;
}

.module-item {
    padding: 20px;
    background-color: #f9f9f9;
    border-radius: 5px;
    border: 1px solid #eee;
}

.module-item h3 {
    margin-top: 0;
}

/* Table of contents */
.toc {
    background-color: #f9f9f9;
    padding: 20px;
    border-radius: 5px;
    margin: 20px 0;
}

.toc h2 {
    margin-top: 0;
}

.toc ul {
    list-style-type: none;
    padding-left: 20px;
}

.toc li {
    margin: 5px 0;
}

/* Documentation items */
.item {
    margin: 30px 0;
    padding: 20px;
    border-left: 4px solid #0066cc;
    background-color: #f9f9f9;
}

.item h3 {
    margin-top: 0;
}

.signature {
    background-color: #fff;
    border: 1px solid #ddd;
    margin: 10px 0;
}

/* Documentation sections */
.documentation {
    margin-top: 15px;
}

.documentation p {
    margin: 10px 0;
}

.params, .returns, .examples, .notes, .see-also {
    margin-top: 20px;
}

.params ul, .notes ul, .see-also ul {
    list-style-type: disc;
    padding-left: 30px;
}

.params li, .notes li, .see-also li {
    margin: 5px 0;
}

.type {
    color: #0066cc;
    font-weight: 500;
}

.output {
    background-color: #333;
    color: #fff;
}

.output code {
    color: #fff;
}

/* Responsive design */
@media (max-width: 768px) {
    .container {
        padding: 10px;
    }
    
    .module-list {
        grid-template-columns: 1fr;
    }
    
    h1 { font-size: 2em; }
    h2 { font-size: 1.5em; }
    h3 { font-size: 1.2em; }
}
"#.to_string()
    }

    /// Default JavaScript for search functionality
    fn default_js() -> String {
        r#"
// Search functionality
document.addEventListener('DOMContentLoaded', function() {
    const searchInput = document.getElementById('search-input');
    const searchResults = document.getElementById('search-results');
    
    if (!searchInput || !searchResults) return;
    
    searchInput.addEventListener('input', function() {
        const query = this.value.toLowerCase().trim();
        
        if (query.length < 2) {
            searchResults.classList.remove('active');
            return;
        }
        
        const results = performSearch(query);
        displayResults(results);
    });
    
    function performSearch(query) {
        const results = [];
        
        for (const term in SEARCH_INDEX) {
            if (term.includes(query)) {
                for (const item of SEARCH_INDEX[term]) {
                    // Avoid duplicates
                    if (!results.some(r => r.path === item.path)) {
                        results.push(item);
                    }
                }
            }
        }
        
        // Sort by relevance (exact matches first)
        results.sort((a, b) => {
            const aExact = a.name.toLowerCase() === query;
            const bExact = b.name.toLowerCase() === query;
            if (aExact && !bExact) return -1;
            if (!aExact && bExact) return 1;
            return a.name.localeCompare(b.name);
        });
        
        return results.slice(0, 20); // Limit to 20 results
    }
    
    function displayResults(results) {
        if (results.length === 0) {
            searchResults.innerHTML = '<p>No results found.</p>';
            searchResults.classList.add('active');
            return;
        }
        
        let html = '<h3>Search Results</h3>';
        
        for (const result of results) {
            const url = result.path.replace('::', '_') + '.html#' + getAnchor(result);
            html += `
                <div class="search-result">
                    <a href="${url}">
                        <strong>${result.name}</strong>
                        <span class="search-kind">(${result.kind})</span>
                    </a>
                    <p>${result.summary}</p>
                </div>
            `;
        }
        
        searchResults.innerHTML = html;
        searchResults.classList.add('active');
    }
    
    function getAnchor(item) {
        switch (item.kind) {
            case 'Function': return 'fn.' + item.name;
            case 'Type': return 'type.' + item.name;
            case 'Constant': return 'const.' + item.name;
            case 'Method': return 'method.' + item.name;
            default: return item.name;
        }
    }
});
"#
        .to_string()
    }
}
