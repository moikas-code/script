use crate::source::SourceLocation;
use std::collections::HashMap;
use std::fmt;

/// Represents a variable in the debugger
#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
    pub name: String,
    pub value: VariableValue,
    pub type_info: String,
    pub scope: VariableScope,
}

/// The value of a variable (simplified for debugging purposes)
#[derive(Debug, Clone, PartialEq)]
pub enum VariableValue {
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Array(Vec<VariableValue>),
    Object(HashMap<String, VariableValue>),
    Function(String), // Function signature as string
    Unknown(String),  // For values we can't represent
}

/// Variable scope information
#[derive(Debug, Clone, PartialEq)]
pub enum VariableScope {
    Local,
    Parameter,
    Global,
    Closure,
}

/// Represents a single frame in the call stack
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Name of the function this frame represents
    function_name: String,
    /// Source location where this function was called
    call_location: Option<SourceLocation>,
    /// Local variables in this frame
    local_variables: HashMap<String, Variable>,
    /// Function parameters (subset of locals, but tracked separately)
    parameters: HashMap<String, Variable>,
    /// Line number where execution is currently paused in this frame
    current_line: Option<usize>,
}

impl StackFrame {
    /// Create a new stack frame
    pub fn new(function_name: String, call_location: Option<SourceLocation>) -> Self {
        Self {
            function_name,
            call_location,
            local_variables: HashMap::new(),
            parameters: HashMap::new(),
            current_line: call_location.map(|loc| loc.line),
        }
    }

    /// Get the function name
    pub fn function_name(&self) -> &str {
        &self.function_name
    }

    /// Get the call location
    pub fn call_location(&self) -> Option<&SourceLocation> {
        self.call_location.as_ref()
    }

    /// Get current line number
    pub fn current_line(&self) -> Option<usize> {
        self.current_line
    }

    /// Set current line number
    pub fn set_current_line(&mut self, line: usize) {
        self.current_line = Some(line);
    }

    /// Add a local variable
    pub fn add_local_variable(&mut self, variable: Variable) {
        self.local_variables.insert(variable.name.clone(), variable);
    }

    /// Add a parameter variable
    pub fn add_parameter(&mut self, variable: Variable) {
        self.parameters.insert(variable.name.clone(), variable.clone());
        self.local_variables.insert(variable.name.clone(), variable);
    }

    /// Get a variable by name (checks both locals and parameters)
    pub fn get_variable(&self, name: &str) -> Option<&Variable> {
        self.local_variables.get(name)
    }

    /// Get all local variables
    pub fn local_variables(&self) -> &HashMap<String, Variable> {
        &self.local_variables
    }

    /// Get all parameters
    pub fn parameters(&self) -> &HashMap<String, Variable> {
        &self.parameters
    }

    /// Update a variable's value
    pub fn update_variable(&mut self, name: &str, value: VariableValue) -> bool {
        if let Some(var) = self.local_variables.get_mut(name) {
            var.value = value.clone();
            // Also update in parameters if it exists there
            if let Some(param) = self.parameters.get_mut(name) {
                param.value = value;
            }
            true
        } else {
            false
        }
    }

    /// Get all variables (combines locals and shows scope info)
    pub fn all_variables(&self) -> Vec<&Variable> {
        self.local_variables.values().collect()
    }

    /// Check if a variable exists in this frame
    pub fn has_variable(&self, name: &str) -> bool {
        self.local_variables.contains_key(name)
    }

    /// Remove a variable from this frame
    pub fn remove_variable(&mut self, name: &str) -> Option<Variable> {
        self.parameters.remove(name);
        self.local_variables.remove(name)
    }

    /// Clear all variables (for frame reuse)
    pub fn clear_variables(&mut self) {
        self.local_variables.clear();
        self.parameters.clear();
    }
}

impl Variable {
    /// Create a new variable
    pub fn new(name: String, value: VariableValue, scope: VariableScope) -> Self {
        let type_info = value.type_name().to_string();
        Self {
            name,
            value,
            type_info,
            scope,
        }
    }

    /// Create a number variable
    pub fn number(name: String, value: f64, scope: VariableScope) -> Self {
        Self::new(name, VariableValue::Number(value), scope)
    }

    /// Create a string variable
    pub fn string(name: String, value: String, scope: VariableScope) -> Self {
        Self::new(name, VariableValue::String(value), scope)
    }

    /// Create a boolean variable
    pub fn boolean(name: String, value: bool, scope: VariableScope) -> Self {
        Self::new(name, VariableValue::Boolean(value), scope)
    }

    /// Create a null variable
    pub fn null(name: String, scope: VariableScope) -> Self {
        Self::new(name, VariableValue::Null, scope)
    }

    /// Get the variable's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the variable's value
    pub fn value(&self) -> &VariableValue {
        &self.value
    }

    /// Get the variable's type information
    pub fn type_info(&self) -> &str {
        &self.type_info
    }

    /// Get the variable's scope
    pub fn scope(&self) -> &VariableScope {
        &self.scope
    }

    /// Update the variable's value
    pub fn set_value(&mut self, value: VariableValue) {
        self.type_info = value.type_name().to_string();
        self.value = value;
    }
}

impl VariableValue {
    /// Get the type name of this value
    pub fn type_name(&self) -> &'static str {
        match self {
            VariableValue::Number(_) => "number",
            VariableValue::String(_) => "string",
            VariableValue::Boolean(_) => "boolean",
            VariableValue::Null => "null",
            VariableValue::Array(_) => "array",
            VariableValue::Object(_) => "object",
            VariableValue::Function(_) => "function",
            VariableValue::Unknown(_) => "unknown",
        }
    }

    /// Check if this is a primitive value
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            VariableValue::Number(_)
                | VariableValue::String(_)
                | VariableValue::Boolean(_)
                | VariableValue::Null
        )
    }

    /// Get a string representation suitable for debugging display
    pub fn debug_string(&self) -> String {
        match self {
            VariableValue::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            VariableValue::String(s) => format!("\"{}\"", s),
            VariableValue::Boolean(b) => b.to_string(),
            VariableValue::Null => "null".to_string(),
            VariableValue::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else if arr.len() <= 5 {
                    let items: Vec<String> = arr.iter().map(|v| v.debug_string()).collect();
                    format!("[{}]", items.join(", "))
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            VariableValue::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{ {} properties }}", obj.len())
                }
            }
            VariableValue::Function(sig) => format!("fn {}", sig),
            VariableValue::Unknown(desc) => format!("<{}>", desc),
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function_name)?;
        if let Some(location) = &self.call_location {
            write!(f, " at {}:{}", location.line, location.column)?;
        }
        if let Some(line) = self.current_line {
            write!(f, " (line {})", line)?;
        }
        Ok(())
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} = {}",
            self.name,
            self.type_info,
            self.value.debug_string()
        )
    }
}

impl fmt::Display for VariableScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope = match self {
            VariableScope::Local => "local",
            VariableScope::Parameter => "parameter",
            VariableScope::Global => "global",
            VariableScope::Closure => "closure",
        };
        write!(f, "{}", scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceLocation;

    #[test]
    fn test_variable_creation() {
        let var = Variable::number("x".to_string(), 42.0, VariableScope::Local);
        assert_eq!(var.name(), "x");
        assert_eq!(var.type_info(), "number");
        assert_eq!(var.value(), &VariableValue::Number(42.0));
        assert_eq!(var.scope(), &VariableScope::Local);
    }

    #[test]
    fn test_variable_value_debug_string() {
        assert_eq!(VariableValue::Number(42.0).debug_string(), "42");
        assert_eq!(VariableValue::Number(3.14).debug_string(), "3.14");
        assert_eq!(
            VariableValue::String("hello".to_string()).debug_string(),
            "\"hello\""
        );
        assert_eq!(VariableValue::Boolean(true).debug_string(), "true");
        assert_eq!(VariableValue::Null.debug_string(), "null");

        let arr = VariableValue::Array(vec![
            VariableValue::Number(1.0),
            VariableValue::Number(2.0),
            VariableValue::Number(3.0),
        ]);
        assert_eq!(arr.debug_string(), "[1, 2, 3]");

        let big_arr = VariableValue::Array(vec![VariableValue::Number(1.0); 10]);
        assert_eq!(big_arr.debug_string(), "[10 items]");
    }

    #[test]
    fn test_stack_frame_variables() {
        let mut frame = StackFrame::new("test_func".to_string(), None);

        // Add a parameter
        let param = Variable::number("param1".to_string(), 10.0, VariableScope::Parameter);
        frame.add_parameter(param);

        // Add a local variable
        let local = Variable::string("local1".to_string(), "hello".to_string(), VariableScope::Local);
        frame.add_local_variable(local);

        // Check variables exist
        assert!(frame.has_variable("param1"));
        assert!(frame.has_variable("local1"));
        assert!(!frame.has_variable("nonexistent"));

        // Check variable retrieval
        let param_var = frame.get_variable("param1").unwrap();
        assert_eq!(param_var.value(), &VariableValue::Number(10.0));

        let local_var = frame.get_variable("local1").unwrap();
        assert_eq!(local_var.value(), &VariableValue::String("hello".to_string()));

        // Update variable
        assert!(frame.update_variable("param1", VariableValue::Number(20.0)));
        let updated_param = frame.get_variable("param1").unwrap();
        assert_eq!(updated_param.value(), &VariableValue::Number(20.0));

        // Check counts
        assert_eq!(frame.parameters().len(), 1);
        assert_eq!(frame.local_variables().len(), 2); // param1 is in both
    }

    #[test]
    fn test_stack_frame_display() {
        let location = SourceLocation::new(10, 5);
        let mut frame = StackFrame::new("test_function".to_string(), Some(location));
        frame.set_current_line(15);

        let display = frame.to_string();
        assert!(display.contains("test_function"));
        assert!(display.contains("10:5"));
        assert!(display.contains("line 15"));
    }

    #[test]
    fn test_variable_types() {
        let num = VariableValue::Number(42.0);
        assert_eq!(num.type_name(), "number");
        assert!(num.is_primitive());

        let arr = VariableValue::Array(vec![]);
        assert_eq!(arr.type_name(), "array");
        assert!(!arr.is_primitive());

        let func = VariableValue::Function("test() -> void".to_string());
        assert_eq!(func.type_name(), "function");
        assert!(!func.is_primitive());
    }

    #[test]
    fn test_variable_scope_display() {
        assert_eq!(VariableScope::Local.to_string(), "local");
        assert_eq!(VariableScope::Parameter.to_string(), "parameter");
        assert_eq!(VariableScope::Global.to_string(), "global");
        assert_eq!(VariableScope::Closure.to_string(), "closure");
    }

    #[test]
    fn test_frame_variable_management() {
        let mut frame = StackFrame::new("test".to_string(), None);

        let var1 = Variable::number("x".to_string(), 1.0, VariableScope::Local);
        let var2 = Variable::string("y".to_string(), "test".to_string(), VariableScope::Local);

        frame.add_local_variable(var1);
        frame.add_local_variable(var2);

        assert_eq!(frame.all_variables().len(), 2);

        // Remove one variable
        let removed = frame.remove_variable("x");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name(), "x");
        assert_eq!(frame.all_variables().len(), 1);

        // Clear all variables
        frame.clear_variables();
        assert_eq!(frame.all_variables().len(), 0);
    }
}