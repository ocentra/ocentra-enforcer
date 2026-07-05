//! Python extraction via `tree-sitter-python`: functions (including
//! `test_*`-named pytest functions and `unittest`-style methods, tagged
//! [`SymbolKind::Test`]), classes, imports, calls, and Flask/FastAPI
//! route decorators.

use crate::parsers::{CallRef, ImportRef, ParsedFile, RouteRef, SymbolKind, SymbolRef};
use tree_sitter::{Node, Parser};

const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "options", "head"];

pub fn parse(source: &str) -> ParsedFile {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return ParsedFile::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedFile::default();
    };

    let mut out = ParsedFile::default();
    walk(tree.root_node(), source.as_bytes(), &mut out);
    out
}

fn walk(node: Node<'_>, src: &[u8], out: &mut ParsedFile) {
    match node.kind() {
        "function_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    kind: test_or_function(&name),
                    name,
                    line: node.start_position().row + 1,
                });
            }
        }
        "class_definition" => {
            if let Some(name) = child_text(node, "name", src) {
                out.symbols.push(SymbolRef {
                    name,
                    kind: SymbolKind::Type,
                    line: node.start_position().row + 1,
                });
            }
        }
        "decorated_definition" => {
            if let Some(route) = route_from_decorated(node, src) {
                out.routes.push(route);
            }
        }
        "import_statement" => {
            for path in dotted_names_under(node, src) {
                out.imports.push(ImportRef {
                    module_path: path,
                    line: node.start_position().row + 1,
                });
            }
        }
        "import_from_statement" => {
            if let Some(module_node) = node.child_by_field_name("module_name") {
                if let Ok(module) = module_node.utf8_text(src) {
                    out.imports.push(ImportRef {
                        module_path: module.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
            }
        }
        "call" => {
            if let Some(function) = node.child_by_field_name("function") {
                out.calls.push(CallRef {
                    callee: function.utf8_text(src).unwrap_or("").to_string(),
                    line: node.start_position().row + 1,
                });
            }
        }
        _ => {}
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            walk(child, src, out);
        }
    }
}

fn test_or_function(name: &str) -> SymbolKind {
    if name.starts_with("test_") || name.starts_with("test") {
        SymbolKind::Test
    } else {
        SymbolKind::Function
    }
}

fn child_text(node: Node<'_>, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .map(str::to_string)
}

fn dotted_names_under(node: Node<'_>, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            match child.kind() {
                "dotted_name" => {
                    if let Ok(text) = child.utf8_text(src) {
                        out.push(text.to_string());
                    }
                }
                "aliased_import" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if let Ok(text) = name_node.utf8_text(src) {
                            out.push(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Recognize Flask/FastAPI-style decorators: `@app.route("/x")`,
/// `@app.get("/x")`, `@router.post("/x")`.
fn route_from_decorated(decorated_node: Node<'_>, src: &[u8]) -> Option<RouteRef> {
    for i in 0..decorated_node.child_count() {
        let child = decorated_node.child(i)?;
        if child.kind() != "decorator" {
            continue;
        }
        // A `decorator` node wraps either a bare `identifier`/
        // `attribute` or a `call` (when it has arguments).
        let call = (0..child.child_count())
            .filter_map(|j| child.child(j))
            .find(|n| n.kind() == "call")?;
        let function = call.child_by_field_name("function")?;
        let function_text = function.utf8_text(src).unwrap_or("");
        let method_word = function_text.rsplit('.').next().unwrap_or("");
        let method = if method_word.eq_ignore_ascii_case("route") {
            "GET".to_string()
        } else if HTTP_METHODS
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(method_word))
        {
            method_word.to_uppercase()
        } else {
            continue;
        };
        let path = call
            .child_by_field_name("arguments")
            .and_then(|args| {
                (0..args.child_count())
                    .filter_map(|k| args.child(k))
                    .find(|n| n.kind() == "string")
            })
            .and_then(|n| n.utf8_text(src).ok())
            .map(|raw| raw.trim_matches(|c| c == '"' || c == '\'').to_string())
            .unwrap_or_default();
        if path.is_empty() {
            continue;
        }
        return Some(RouteRef {
            method,
            path,
            line: decorated_node.start_position().row + 1,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_function_and_test_symbols() {
        let src = "def normal():\n    pass\n\ndef test_something():\n    pass\n";
        let parsed = parse(src);
        let names_kinds: Vec<(&str, SymbolKind)> = parsed
            .symbols
            .iter()
            .map(|s| (s.name.as_str(), s.kind))
            .collect();
        assert!(names_kinds.contains(&("normal", SymbolKind::Function)));
        assert!(names_kinds.contains(&("test_something", SymbolKind::Test)));
    }

    #[test]
    fn extracts_class_as_type() {
        let src = "class Foo:\n    pass\n";
        let parsed = parse(src);
        assert!(parsed
            .symbols
            .iter()
            .any(|s| s.name == "Foo" && s.kind == SymbolKind::Type));
    }

    #[test]
    fn extracts_imports() {
        let src = "import os\nfrom typing import List\n";
        let parsed = parse(src);
        let paths: Vec<&str> = parsed
            .imports
            .iter()
            .map(|i| i.module_path.as_str())
            .collect();
        assert!(paths.contains(&"os"));
        assert!(paths.contains(&"typing"));
    }

    #[test]
    fn extracts_call_edges() {
        let src = "def f():\n    helper()\n    ns.thing()\n";
        let parsed = parse(src);
        let callees: Vec<&str> = parsed.calls.iter().map(|c| c.callee.as_str()).collect();
        assert!(callees.contains(&"helper"));
        assert!(callees.contains(&"ns.thing"));
    }

    #[test]
    fn extracts_flask_route_decorator() {
        let src = "@app.route(\"/hello\")\ndef hello():\n    pass\n";
        let parsed = parse(src);
        assert_eq!(parsed.routes.len(), 1);
        assert_eq!(parsed.routes[0].method, "GET");
        assert_eq!(parsed.routes[0].path, "/hello");
    }

    #[test]
    fn extracts_fastapi_post_decorator() {
        let src = "@router.post(\"/items\")\ndef create():\n    pass\n";
        let parsed = parse(src);
        assert!(parsed
            .routes
            .iter()
            .any(|r| r.method == "POST" && r.path == "/items"));
    }
}
