//! Per-language tree-sitter extraction. Each submodule owns one
//! grammar and turns its parse tree into the language-agnostic
//! [`crate::parsers::ParsedFile`] shape that [`crate::code_graph`]
//! turns into graph nodes/edges. Adding a new language means adding a
//! new submodule here plus one dispatch arm in
//! [`crate::parsers::parse_file`] -- [`crate::code_graph`] itself never
//! needs to know which grammar produced a [`crate::parsers::ParsedFile`].

pub mod c;
pub mod cpp;
pub mod csharp;
pub mod generic;
pub mod go;
pub mod java;
pub mod php;
pub mod python;
pub mod rust;
pub mod spec;
pub mod typescript;
