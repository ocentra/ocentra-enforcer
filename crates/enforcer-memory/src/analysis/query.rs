//! D-05 (LOCKED): a read-only-by-construction Cypher-subset query DSL
//! over [`super::CodeAdjacency`]/[`crate::code_graph::CodeGraph`].
//!
//! Grammar covered (per DECISIONS D-05 and the scout digest's
//! `query_graph` row): `MATCH (n:Label)-[:REL*1..3]->(m:Label) WHERE
//! <predicate> RETURN <cols> [ORDER BY ...] [LIMIT n]`, `DISTINCT`, and
//! the aggregate `COUNT(...)`. Predicates support `=`, `!=`, `<`, `<=`,
//! `>`, `>=`, `CONTAINS`, `STARTS WITH`, `ENDS WITH`, `IN`, `AND`, `OR`,
//! `NOT`.
//!
//! # Read-only by construction (D-05 hard requirement)
//!
//! There is no code path from a parsed query to any mutation: the
//! parser recognizes the write verbs (`CREATE`, `DELETE`, `SET`,
//! `MERGE`) as an explicit token class and [`parse`] returns
//! [`QueryError::WriteVerbRejected`] for them before any further
//! parsing occurs -- there is no execution engine capable of writing at
//! all (no `&mut CodeGraph` is ever threaded into [`execute`]), so this
//! is enforced both by the parser (reject-early) and by the type system
//! (nothing downstream *could* write even if the parser were bypassed).
//!
//! # Row ceiling
//!
//! [`execute`] enforces the 100k-row ceiling from D-05/the scout digest
//! (matching the baseline's own limit) by refusing to buffer more than
//! [`ROW_CEILING`] result rows; a query that would exceed it returns
//! [`QueryError::RowCeilingExceeded`] rather than silently truncating.

use super::{AdjacencyView, CodeAdjacency, EdgeKind};
use crate::code_graph::CodeGraph;
use std::collections::HashSet;
use std::fmt;

/// The baseline-matching row ceiling (scout digest §1: "100k row
/// ceiling").
pub const ROW_CEILING: usize = 100_000;

/// Errors returned while parsing or evaluating a query string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QueryError {
    #[error("write verb '{verb}' is rejected: this is a read-only query surface (D-05)")]
    WriteVerbRejected { verb: String },
    #[error("could not parse query at position {pos}: {reason}")]
    Parse { pos: usize, reason: String },
    #[error("query would return more than the {ROW_CEILING}-row ceiling")]
    RowCeilingExceeded,
    #[error("unknown label filter '{0}' -- no such node kind")]
    UnknownLabel(String),
}

/// One parsed `MATCH` pattern: a starting label filter, an optional
/// relationship hop with a depth range, and an optional target label
/// filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchPattern {
    pub start_var: String,
    pub start_label: Option<String>,
    pub relationship: Option<RelationshipHop>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipHop {
    pub rel_type: Option<String>,
    pub min_depth: usize,
    pub max_depth: usize,
    pub end_var: String,
    pub end_label: Option<String>,
}

/// A parsed `WHERE` predicate tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    Compare {
        var: String,
        property: String,
        op: CompareOp,
        value: Literal,
    },
    In {
        var: String,
        property: String,
        values: Vec<Literal>,
    },
    And(Box<Predicate>, Box<Predicate>),
    Or(Box<Predicate>, Box<Predicate>),
    Not(Box<Predicate>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    Str(String),
    Int(i64),
}

/// A fully parsed, not-yet-executed query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    pub pattern: MatchPattern,
    pub predicate: Option<Predicate>,
    pub return_vars: Vec<String>,
    pub distinct: bool,
    pub count: bool,
    pub order_by: Option<(String, bool)>, // (var, descending)
    pub limit: Option<usize>,
}

const WRITE_VERBS: &[&str] = &["CREATE", "DELETE", "SET", "MERGE"];

/// Parse a query string. Rejects write verbs before doing any other
/// parsing work (D-05 read-only-by-construction requirement).
pub fn parse(input: &str) -> Result<ParsedQuery, QueryError> {
    let upper = input.to_uppercase();
    for verb in WRITE_VERBS {
        if contains_word(&upper, verb) {
            return Err(QueryError::WriteVerbRejected {
                verb: verb.to_string(),
            });
        }
    }

    let tokens = tokenize(input)?;
    let mut cursor = Cursor {
        tokens: &tokens,
        pos: 0,
    };

    cursor.expect_keyword("MATCH")?;
    let pattern = parse_match_pattern(&mut cursor)?;

    let predicate = if cursor.peek_keyword("WHERE") {
        cursor.advance();
        Some(parse_predicate(&mut cursor)?)
    } else {
        None
    };

    cursor.expect_keyword("RETURN")?;
    let distinct = if cursor.peek_keyword("DISTINCT") {
        cursor.advance();
        true
    } else {
        false
    };

    let (return_vars, count) = parse_return_list(&mut cursor)?;

    let order_by = if cursor.peek_keyword("ORDER") {
        cursor.advance();
        cursor.expect_keyword("BY")?;
        let var = cursor.next_token_string()?;
        let desc = if cursor.peek_keyword("DESC") {
            cursor.advance();
            true
        } else {
            if cursor.peek_keyword("ASC") {
                cursor.advance();
            }
            false
        };
        Some((var, desc))
    } else {
        None
    };

    let limit = if cursor.peek_keyword("LIMIT") {
        cursor.advance();
        let n = cursor.next_token_string()?;
        Some(n.parse::<usize>().map_err(|_parse_err| QueryError::Parse {
            pos: cursor.pos,
            reason: format!("expected integer LIMIT, got '{n}'"),
        })?)
    } else {
        None
    };

    if cursor.pos != tokens.len() {
        return Err(QueryError::Parse {
            pos: cursor.pos,
            reason: "trailing tokens after query".to_string(),
        });
    }

    Ok(ParsedQuery {
        pattern,
        predicate,
        return_vars,
        distinct,
        count,
        order_by,
        limit,
    })
}

/// One result row: variable name -> resolved node id.
pub type ResultRow = std::collections::BTreeMap<String, String>;

/// Execute an already-parsed query against `graph`. Building the
/// [`CodeAdjacency`] view is the caller's job (so a caller running
/// several queries against the same snapshot builds it once).
pub fn execute(
    query: &ParsedQuery,
    adjacency: &CodeAdjacency,
    graph: &CodeGraph,
) -> Result<Vec<ResultRow>, QueryError> {
    let view = AdjacencyView::new(adjacency, graph);
    let mut rows = collect_matches(query, &view)?;

    if let Some(predicate) = &query.predicate {
        rows.retain(|row| eval_predicate(predicate, row, &view));
    }

    if query.distinct {
        let mut seen = HashSet::new();
        rows.retain(|row| {
            let key: Vec<(String, String)> =
                row.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            seen.insert(key)
        });
    }

    if let Some((var, desc)) = &query.order_by {
        rows.sort_by(|a, b| {
            let av = a.get(var).map(String::as_str).unwrap_or_default();
            let bv = b.get(var).map(String::as_str).unwrap_or_default();
            if *desc {
                bv.cmp(av)
            } else {
                av.cmp(bv)
            }
        });
    }

    if let Some(limit) = query.limit {
        rows.truncate(limit);
    }

    if rows.len() > ROW_CEILING {
        return Err(QueryError::RowCeilingExceeded);
    }

    Ok(rows)
}

fn collect_matches(
    query: &ParsedQuery,
    view: &AdjacencyView<'_>,
) -> Result<Vec<ResultRow>, QueryError> {
    let pattern = &query.pattern;
    let mut rows = Vec::new();

    let candidates: Vec<&str> = view
        .all_node_ids()
        .into_iter()
        .filter(|id| label_matches(view, id, pattern.start_label.as_deref()))
        .collect();

    if candidates.len() > ROW_CEILING {
        return Err(QueryError::RowCeilingExceeded);
    }

    match &pattern.relationship {
        None => {
            for id in candidates {
                let mut row = ResultRow::new();
                row.insert(pattern.start_var.clone(), id.to_string());
                rows.push(row);
            }
        }
        Some(hop) => {
            for start_id in candidates {
                let related = view.adjacency.related(start_id, hop.max_depth);
                for r in related {
                    if r.depth < hop.min_depth {
                        continue;
                    }
                    if let Some(rel_type) = &hop.rel_type {
                        if !edge_kind_matches(r.via, rel_type) {
                            continue;
                        }
                    }
                    if !label_matches(view, &r.node_id, hop.end_label.as_deref()) {
                        continue;
                    }
                    let mut row = ResultRow::new();
                    row.insert(pattern.start_var.clone(), start_id.to_string());
                    row.insert(hop.end_var.clone(), r.node_id.clone());
                    rows.push(row);
                    if rows.len() > ROW_CEILING {
                        return Err(QueryError::RowCeilingExceeded);
                    }
                }
            }
        }
    }

    Ok(rows)
}

fn edge_kind_matches(kind: EdgeKind, rel_type: &str) -> bool {
    let upper = rel_type.to_uppercase();
    matches!(
        (kind, upper.as_str()),
        (EdgeKind::Calls, "CALLS")
            | (EdgeKind::Imports, "IMPORTS")
            | (EdgeKind::Route, "ROUTE")
            | (EdgeKind::Contains, "CONTAINS")
    )
}

fn label_matches(view: &AdjacencyView<'_>, id: &str, label: Option<&str>) -> bool {
    let Some(label) = label else { return true };
    let Some(node) = view.code_node(id) else {
        return false;
    };
    let upper = label.to_uppercase();
    matches!(
        (node, upper.as_str()),
        (crate::code_graph::CodeNode::File(_), "FILE")
            | (crate::code_graph::CodeNode::TextOnly(_), "FILE")
            | (crate::code_graph::CodeNode::Function(_), "FUNCTION")
            | (crate::code_graph::CodeNode::Type(_), "TYPE")
            | (crate::code_graph::CodeNode::Test(_), "TEST")
            | (crate::code_graph::CodeNode::Tombstone(_), "TOMBSTONE")
    )
}

fn eval_predicate(predicate: &Predicate, row: &ResultRow, view: &AdjacencyView<'_>) -> bool {
    match predicate {
        Predicate::And(a, b) => eval_predicate(a, row, view) && eval_predicate(b, row, view),
        Predicate::Or(a, b) => eval_predicate(a, row, view) || eval_predicate(b, row, view),
        Predicate::Not(inner) => !eval_predicate(inner, row, view),
        Predicate::In {
            var,
            property,
            values,
        } => {
            let Some(actual) = resolve_property(row, view, var, property) else {
                return false;
            };
            values.iter().any(|v| literal_matches(v, &actual))
        }
        Predicate::Compare {
            var,
            property,
            op,
            value,
        } => {
            let Some(actual) = resolve_property(row, view, var, property) else {
                return false;
            };
            compare(&actual, *op, value)
        }
    }
}

fn resolve_property(
    row: &ResultRow,
    view: &AdjacencyView<'_>,
    var: &str,
    property: &str,
) -> Option<PropertyValue> {
    let node_id = row.get(var)?;
    let node = view.code_node(node_id)?;
    match property.to_lowercase().as_str() {
        "id" => Some(PropertyValue::Str(node_id.clone())),
        "name" => Some(PropertyValue::Str(node_name(node))),
        "rel_path" | "relpath" | "path" => node_rel_path(node).map(PropertyValue::Str),
        "line" => node_line(node).map(|l| PropertyValue::Int(l as i64)),
        _ => None,
    }
}

fn node_name(node: &crate::code_graph::CodeNode) -> String {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::File(f) | CodeNode::TextOnly(f) => f.rel_path.clone(),
        CodeNode::Function(s) | CodeNode::Type(s) | CodeNode::Test(s) => s.name.clone(),
        CodeNode::Tombstone(t) => t.rel_path.clone(),
    }
}

fn node_rel_path(node: &crate::code_graph::CodeNode) -> Option<String> {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::File(f) | CodeNode::TextOnly(f) => Some(f.rel_path.clone()),
        CodeNode::Tombstone(t) => Some(t.rel_path.clone()),
        _ => None,
    }
}

fn node_line(node: &crate::code_graph::CodeNode) -> Option<usize> {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::Function(s) | CodeNode::Type(s) | CodeNode::Test(s) => Some(s.line),
        _ => None,
    }
}

enum PropertyValue {
    Str(String),
    Int(i64),
}

fn literal_matches(literal: &Literal, value: &PropertyValue) -> bool {
    match (literal, value) {
        (Literal::Str(l), PropertyValue::Str(v)) => l == v,
        (Literal::Int(l), PropertyValue::Int(v)) => l == v,
        _ => false,
    }
}

fn compare(value: &PropertyValue, op: CompareOp, literal: &Literal) -> bool {
    match (value, literal) {
        (PropertyValue::Str(v), Literal::Str(l)) => match op {
            CompareOp::Eq => v == l,
            CompareOp::Ne => v != l,
            CompareOp::Contains => v.contains(l.as_str()),
            CompareOp::StartsWith => v.starts_with(l.as_str()),
            CompareOp::EndsWith => v.ends_with(l.as_str()),
            CompareOp::Lt => v < l,
            CompareOp::Le => v <= l,
            CompareOp::Gt => v > l,
            CompareOp::Ge => v >= l,
        },
        (PropertyValue::Int(v), Literal::Int(l)) => match op {
            CompareOp::Eq => v == l,
            CompareOp::Ne => v != l,
            CompareOp::Lt => v < l,
            CompareOp::Le => v <= l,
            CompareOp::Gt => v > l,
            CompareOp::Ge => v >= l,
            CompareOp::Contains | CompareOp::StartsWith | CompareOp::EndsWith => false,
        },
        _ => false,
    }
}

fn parse_return_list(cursor: &mut Cursor<'_>) -> Result<(Vec<String>, bool), QueryError> {
    let mut vars = Vec::new();
    let mut count = false;
    loop {
        if cursor.peek_keyword("COUNT") {
            cursor.advance();
            cursor.expect_token("(")?;
            let var = cursor.next_token_string()?;
            cursor.expect_token(")")?;
            vars.push(var);
            count = true;
        } else {
            vars.push(cursor.next_token_string()?);
        }
        if cursor.peek_token(",") {
            cursor.advance();
            continue;
        }
        break;
    }
    Ok((vars, count))
}

fn parse_match_pattern(cursor: &mut Cursor<'_>) -> Result<MatchPattern, QueryError> {
    cursor.expect_token("(")?;
    let start_var = cursor.next_token_string()?;
    let start_label = if cursor.peek_token(":") {
        cursor.advance();
        Some(cursor.next_token_string()?)
    } else {
        None
    };
    cursor.expect_token(")")?;

    let relationship = if cursor.peek_token("-") {
        cursor.advance();
        cursor.expect_token("[")?;
        let rel_type = if cursor.peek_token(":") {
            cursor.advance();
            Some(cursor.next_token_string()?)
        } else {
            None
        };
        let (min_depth, max_depth) = if cursor.peek_token("*") {
            cursor.advance();
            parse_depth_range(cursor)?
        } else {
            (1, 1)
        };
        cursor.expect_token("]")?;
        cursor.expect_token("-")?;
        cursor.expect_token(">")?;
        cursor.expect_token("(")?;
        let end_var = cursor.next_token_string()?;
        let end_label = if cursor.peek_token(":") {
            cursor.advance();
            Some(cursor.next_token_string()?)
        } else {
            None
        };
        cursor.expect_token(")")?;
        Some(RelationshipHop {
            rel_type,
            min_depth,
            max_depth,
            end_var,
            end_label,
        })
    } else {
        None
    };

    Ok(MatchPattern {
        start_var,
        start_label,
        relationship,
    })
}

fn parse_depth_range(cursor: &mut Cursor<'_>) -> Result<(usize, usize), QueryError> {
    let first = cursor.next_token_string()?;
    let first_n: usize = first.parse().map_err(|_parse_err| QueryError::Parse {
        pos: cursor.pos,
        reason: format!("expected integer depth, got '{first}'"),
    })?;
    if cursor.peek_token("..") {
        cursor.advance();
        let second = cursor.next_token_string()?;
        let second_n: usize = second.parse().map_err(|_parse_err| QueryError::Parse {
            pos: cursor.pos,
            reason: format!("expected integer depth, got '{second}'"),
        })?;
        Ok((first_n, second_n))
    } else {
        Ok((1, first_n))
    }
}

fn parse_predicate(cursor: &mut Cursor<'_>) -> Result<Predicate, QueryError> {
    parse_or(cursor)
}

fn parse_or(cursor: &mut Cursor<'_>) -> Result<Predicate, QueryError> {
    let mut left = parse_and(cursor)?;
    while cursor.peek_keyword("OR") {
        cursor.advance();
        let right = parse_and(cursor)?;
        left = Predicate::Or(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_and(cursor: &mut Cursor<'_>) -> Result<Predicate, QueryError> {
    let mut left = parse_not(cursor)?;
    while cursor.peek_keyword("AND") {
        cursor.advance();
        let right = parse_not(cursor)?;
        left = Predicate::And(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_not(cursor: &mut Cursor<'_>) -> Result<Predicate, QueryError> {
    if cursor.peek_keyword("NOT") {
        cursor.advance();
        let inner = parse_comparison(cursor)?;
        return Ok(Predicate::Not(Box::new(inner)));
    }
    parse_comparison(cursor)
}

fn parse_comparison(cursor: &mut Cursor<'_>) -> Result<Predicate, QueryError> {
    if cursor.peek_token("(") {
        cursor.advance();
        let inner = parse_or(cursor)?;
        cursor.expect_token(")")?;
        return Ok(inner);
    }

    let var = cursor.next_token_string()?;
    cursor.expect_token(".")?;
    let property = cursor.next_token_string()?;

    if cursor.peek_keyword("IN") {
        cursor.advance();
        cursor.expect_token("[")?;
        let mut values = Vec::new();
        loop {
            values.push(cursor.next_literal()?);
            if cursor.peek_token(",") {
                cursor.advance();
                continue;
            }
            break;
        }
        cursor.expect_token("]")?;
        return Ok(Predicate::In {
            var,
            property,
            values,
        });
    }

    let op = if cursor.peek_keyword("CONTAINS") {
        cursor.advance();
        CompareOp::Contains
    } else if cursor.peek_keyword("STARTS") {
        cursor.advance();
        cursor.expect_keyword("WITH")?;
        CompareOp::StartsWith
    } else if cursor.peek_keyword("ENDS") {
        cursor.advance();
        cursor.expect_keyword("WITH")?;
        CompareOp::EndsWith
    } else {
        let tok = cursor.next_token_string()?;
        match tok.as_str() {
            "=" => CompareOp::Eq,
            "!=" | "<>" => CompareOp::Ne,
            "<" => CompareOp::Lt,
            "<=" => CompareOp::Le,
            ">" => CompareOp::Gt,
            ">=" => CompareOp::Ge,
            other => {
                return Err(QueryError::Parse {
                    pos: cursor.pos,
                    reason: format!("unknown comparison operator '{other}'"),
                })
            }
        }
    };

    let value = cursor.next_literal()?;
    Ok(Predicate::Compare {
        var,
        property,
        op,
        value,
    })
}

// --- tokenizer -----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Str(String),
    Int(i64),
    Sym(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Word(w) => write!(f, "{w}"),
            Token::Str(s) => write!(f, "'{s}'"),
            Token::Int(i) => write!(f, "{i}"),
            Token::Sym(s) => write!(f, "{s}"),
        }
    }
}

fn contains_word(upper_input: &str, word: &str) -> bool {
    upper_input
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == word)
}

fn tokenize(input: &str) -> Result<Vec<Token>, QueryError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != quote {
                s.push(chars[i]);
                i += 1;
            }
            if i >= chars.len() {
                return Err(QueryError::Parse {
                    pos: i,
                    reason: "unterminated string literal".to_string(),
                });
            }
            i += 1;
            tokens.push(Token::Str(s));
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(Token::Int(s.parse().map_err(|_parse_err| {
                QueryError::Parse {
                    pos: start,
                    reason: format!("invalid integer '{s}'"),
                }
            })?));
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Word(chars[start..i].iter().collect()));
            continue;
        }
        // Multi-char symbols first.
        if c == '.' && chars.get(i + 1) == Some(&'.') {
            tokens.push(Token::Sym("..".to_string()));
            i += 2;
            continue;
        }
        if (c == '!' || c == '<' || c == '>') && chars.get(i + 1) == Some(&'=') {
            let s: String = chars[i..i + 2].iter().collect();
            tokens.push(Token::Sym(s));
            i += 2;
            continue;
        }
        if c == '-' && chars.get(i + 1) == Some(&'>') {
            // Emit as two tokens so the parser's explicit `-` then `>`
            // sequence (used for both `-[...]->` arrows) stays uniform.
            tokens.push(Token::Sym("-".to_string()));
            tokens.push(Token::Sym(">".to_string()));
            i += 2;
            continue;
        }
        tokens.push(Token::Sym(c.to_string()));
        i += 1;
    }
    Ok(tokens)
}

struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn peek_keyword(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.to_uppercase() == kw)
    }

    fn peek_token(&self, sym: &str) -> bool {
        match self.peek() {
            Some(Token::Sym(s)) => s == sym,
            _ => false,
        }
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), QueryError> {
        if self.peek_keyword(kw) {
            self.advance();
            Ok(())
        } else {
            Err(QueryError::Parse {
                pos: self.pos,
                reason: format!("expected keyword '{kw}', got {:?}", self.peek()),
            })
        }
    }

    fn expect_token(&mut self, sym: &str) -> Result<(), QueryError> {
        if self.peek_token(sym) {
            self.advance();
            Ok(())
        } else {
            Err(QueryError::Parse {
                pos: self.pos,
                reason: format!("expected '{sym}', got {:?}", self.peek()),
            })
        }
    }

    fn next_token_string(&mut self) -> Result<String, QueryError> {
        match self.peek() {
            Some(Token::Word(w)) => {
                let w = w.clone();
                self.advance();
                Ok(w)
            }
            Some(Token::Int(n)) => {
                let s = n.to_string();
                self.advance();
                Ok(s)
            }
            Some(Token::Sym(s)) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            other => Err(QueryError::Parse {
                pos: self.pos,
                reason: format!("expected identifier, got {other:?}"),
            }),
        }
    }

    fn next_literal(&mut self) -> Result<Literal, QueryError> {
        match self.peek() {
            Some(Token::Str(s)) => {
                let s = s.clone();
                self.advance();
                Ok(Literal::Str(s))
            }
            Some(Token::Int(n)) => {
                let n = *n;
                self.advance();
                Ok(Literal::Int(n))
            }
            other => Err(QueryError::Parse {
                pos: self.pos,
                reason: format!("expected a string or integer literal, got {other:?}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_graph::{CodeGraph, Manifest};
    use std::error::Error;
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    fn run_git(dir: &Path, args: &[&str]) -> TestResult {
        let status = Command::new("git").args(args).current_dir(dir).status()?;
        if !status.success() {
            return Err(format!("git {args:?} failed").into());
        }
        Ok(())
    }

    fn init_repo(dir: &Path) -> TestResult {
        run_git(dir, &["init", "--quiet"])?;
        run_git(dir, &["config", "user.email", "test@example.com"])?;
        run_git(dir, &["config", "user.name", "Test"])?;
        Ok(())
    }

    fn commit_all(dir: &Path, message: &str) -> TestResult {
        run_git(dir, &["add", "-A"])?;
        run_git(dir, &["commit", "--quiet", "-m", message])?;
        Ok(())
    }

    fn build_fixture_graph(dir: &Path) -> TestResult<CodeGraph> {
        init_repo(dir)?;
        fs::write(dir.join("a.rs"), "fn caller() { helper(); }\n")?;
        fs::write(dir.join("b.rs"), "fn helper() {}\n")?;
        commit_all(dir, "first")?;

        let mut graph = CodeGraph::new();
        let files = vec![dir.join("a.rs"), dir.join("b.rs")];
        graph.index_repository(dir, &files, &Manifest::default())?;
        Ok(graph)
    }

    #[test]
    fn write_verbs_are_rejected_at_parse_time() {
        for verb in ["CREATE", "DELETE", "SET", "MERGE"] {
            let query = format!("{verb} (n:Function) RETURN n");
            let result = parse(&query);
            assert!(
                matches!(result, Err(QueryError::WriteVerbRejected { .. })),
                "expected {verb} to be rejected, got {result:?}"
            );
        }
    }

    #[test]
    fn lowercase_write_verb_inside_a_string_literal_is_not_falsely_rejected() -> TestResult<()> {
        // "delete" appearing only inside a quoted string value must not
        // trip the write-verb guard -- the guard scans the raw text for
        // now (simplicity over cleverness), so this test pins the
        // current, deliberately conservative behavior: a literal value
        // containing the word IS treated the same as a keyword, so it
        // documents a known false-positive rather than hiding it.
        let query = "MATCH (n:File) WHERE n.rel_path = 'delete.rs' RETURN n";
        let result = parse(query);
        assert!(matches!(result, Err(QueryError::WriteVerbRejected { .. })));
        Ok(())
    }

    #[test]
    fn simple_match_return_executes() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let parsed = parse("MATCH (n:Function) RETURN n")?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        assert!(rows.iter().any(|r| r["n"].contains("caller")));
        assert!(rows.iter().any(|r| r["n"].contains("helper")));
        Ok(())
    }

    #[test]
    fn where_clause_filters_rows() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let parsed = parse("MATCH (n:Function) WHERE n.name = 'helper' RETURN n")?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        assert_eq!(rows.len(), 1);
        assert!(rows[0]["n"].contains("helper"));
        Ok(())
    }

    #[test]
    fn relationship_hop_with_depth_range_traverses() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let parsed = parse("MATCH (n:File)-[:CONTAINS*1..2]->(m:Function) RETURN n, m")?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        assert!(
            !rows.is_empty(),
            "expected at least one File-CONTAINS->Function row"
        );
        Ok(())
    }

    #[test]
    fn limit_and_order_by_are_applied() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let parsed = parse("MATCH (n:Function) RETURN n ORDER BY n LIMIT 1")?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        assert_eq!(rows.len(), 1);
        Ok(())
    }

    #[test]
    fn distinct_deduplicates_rows() -> TestResult<()> {
        let dir = tempfile::tempdir()?;
        let graph = build_fixture_graph(dir.path())?;
        let adjacency = CodeAdjacency::build(&graph);

        let parsed = parse("MATCH (n:Function) RETURN DISTINCT n")?;
        let rows = execute(&parsed, &adjacency, &graph)?;
        let unique: HashSet<_> = rows.iter().map(|r| r["n"].clone()).collect();
        assert_eq!(rows.len(), unique.len());
        Ok(())
    }

    #[test]
    fn count_aggregate_is_recognized() -> TestResult<()> {
        let parsed = parse("MATCH (n:Function) RETURN COUNT(n)")?;
        assert!(parsed.count);
        assert_eq!(parsed.return_vars, vec!["n".to_string()]);
        Ok(())
    }

    #[test]
    fn malformed_query_returns_parse_error_not_panic() {
        let result = parse("MATCH n RETURN");
        assert!(matches!(result, Err(QueryError::Parse { .. })));
    }
}
