// Utilities for analyzing artifact code snippets

// Check if code is a full React component
pub fn is_react_component(code: &str) -> bool {
    let has_import = code.contains("import") && (code.contains("react") || code.contains("React"));
    let has_function_component = code.contains("function ") && code.contains("return");
    let has_arrow_component = code.contains("const ") && code.contains("=>") && code.contains("return");
    let has_export = code.contains("export");
    let has_jsx_return = code.contains("return (") || code.contains("return <");

    (has_import || has_export) && (has_function_component || has_arrow_component) && has_jsx_return
}

// Extract React component name
pub fn extract_component_name(code: &str) -> Option<String> {
    use regex::Regex;

    if let Ok(re) = Regex::new(r"function\s+([A-Z][a-zA-Z0-9_]*)\s*\(") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) { return Some(name.as_str().to_string()); }
        }
    }
    if let Ok(re) = Regex::new(r"const\s+([A-Z][a-zA-Z0-9_]*)\s*[:=]") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) { return Some(name.as_str().to_string()); }
        }
    }
    if let Ok(re) = Regex::new(r"export\s+(?:default\s+)?(?:function\s+)?([A-Z][a-zA-Z0-9_]*)") {
        if let Some(caps) = re.captures(code) {
            if let Some(name) = caps.get(1) { return Some(name.as_str().to_string()); }
        }
    }
    None
}

// Check if code is a full Vue SFC component
pub fn is_vue_component(code: &str) -> bool {
    let has_template = code.contains("<template>");
    let has_script = code.contains("<script");
    let has_setup = code.contains("setup") || code.contains("defineComponent");
    let has_export_default = code.contains("export default");
    has_template && has_script && (has_setup || has_export_default)
}

// Extract Vue component name
pub fn extract_vue_component_name(code: &str) -> Option<String> {
    use regex::Regex;
    if let Ok(re) = Regex::new(r#"name\s*:\s*['"]([A-Z][a-zA-Z0-9_]*)['"]"#) {
        if let Some(caps) = re.captures(code) { if let Some(name) = caps.get(1) { return Some(name.as_str().to_string()); } }
    }
    if let Ok(re) = Regex::new(r#"defineComponent\s*\(\s*\{\s*name\s*:\s*['"]([A-Z][a-zA-Z0-9_]*)['"]"#) {
        if let Some(caps) = re.captures(code) { if let Some(name) = caps.get(1) { return Some(name.as_str().to_string()); } }
    }
    None
}
