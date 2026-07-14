//! D-05 (LOCKED): a read-only-by-construction Cypher-subset query DSL
//! over [`super::CodeAdjacency`]/[`crate::code_graph::CodeGraph`].
//!
//! Grammar covered (per DECISIONS D-05 and the scout digest's
//! `query_graph` row): `MATCH (n:Label)-[:REL*1..3]->(m:Label) WHERE
//! <predicate> RETURN <cols> [ORDER BY ...] [LIMIT n]`, `DISTINCT`, and
//! the aggregate `COUNT(...)`. Predicates support `=`, `!=`, `<`, `<=`,
//! `>`, `>=`, `CONTAINS`, `STARTS WITH`, `ENDS WITH`, `IN`, `AND`, `OR`,
//! `NOT`. `RETURN` and `ORDER BY` columns accept both a bare pattern
//! variable (`n`) and a dotted property access on one (`n.name`,
//! `n.rel_path`, `n.line`, ...) -- the same `var.property` form `WHERE`
//! already accepted, so `MATCH (f:Function) RETURN f.name ORDER BY
//! f.name` (the baseline-class query this parser was failing on) parses
//! and orders by the resolved property value, not the raw node id.
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

/// A single `RETURN`/`ORDER BY` column reference: either a bare pattern
/// variable (`n`) or a dotted property access on one (`n.name`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnRef {
    pub var: String,
    pub property: Option<String>,
}

impl ColumnRef {
    pub fn bare(var: String) -> Self {
        ColumnRef {
            var,
            property: None,
        }
    }
}

impl fmt::Display for ColumnRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.property {
            Some(prop) => write!(f, "{}.{}", self.var, prop),
            None => write!(f, "{}", self.var),
        }
    }
}

/// A fully parsed, not-yet-executed query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    pub pattern: MatchPattern,
    pub predicate: Option<Predicate>,
    pub return_vars: Vec<ColumnRef>,
    pub distinct: bool,
    pub count: bool,
    pub order_by: Option<(ColumnRef, bool)>, // (column, descending)
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
                // ALLOC-JUSTIFICATION: this typed error outlives the borrowed
                // query input, so its rejected verb is retained as owned data.
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
        let column = parse_column_ref(&mut cursor)?;
        let desc = if cursor.peek_keyword("DESC") {
            cursor.advance();
            true
        } else {
            if cursor.peek_keyword("ASC") {
                cursor.advance();
            }
            false
        };
        Some((column, desc))
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
            // ALLOC-JUSTIFICATION: QueryError owns its diagnostic text after
            // parsing returns and cannot borrow the source query.
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
            // CLONE-JUSTIFICATION: the distinct set owns a stable key while
            // rows remain available to the caller after deduplication.
            let key: Vec<(String, String)> =
                row.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            seen.insert(key)
        });
    }

    if let Some((column, desc)) = &query.order_by {
        rows.sort_by(|a, b| {
            let av = order_by_key(column, a, &view);
            let bv = order_by_key(column, b, &view);
            if *desc {
                bv.cmp(&av)
            } else {
                av.cmp(&bv)
            }
        });
    }
    // Note: `order_by_key` returns an `OrderKey` (numeric-aware) rather
    // than a bare `String` -- see its doc comment for why a plain
    // lexicographic string sort is wrong for e.g.
    // `ORDER BY f.transitive_loop_depth DESC` (`"20" < "3"` as
    // strings).

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
                // CLONE-JUSTIFICATION: ResultRow owns keys independently of
                // the parsed query that supplies its variable name.
                // ALLOC-JUSTIFICATION: ResultRow's public value contract is
                // owned String data, independent of this borrowed node id.
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
                    // CLONE-JUSTIFICATION: ResultRow owns keys and values
                    // after this borrowed graph traversal has completed.
                    // ALLOC-JUSTIFICATION: node identifiers become owned
                    // ResultRow values that outlive the traversal borrow.
                    row.insert(pattern.start_var.clone(), start_id.to_string());
                    // CLONE-JUSTIFICATION: ResultRow owns the endpoint name
                    // after the borrowed traversal result is dropped.
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
            | (EdgeKind::Inherits, "INHERITS")
            | (EdgeKind::Implements, "IMPLEMENTS")
            | (EdgeKind::Decorates, "DECORATES")
            | (EdgeKind::TypeRef, "TYPE_REF")
            | (EdgeKind::Defines, "DEFINES")
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
            | (crate::code_graph::CodeNode::Method(_), "METHOD")
            | (crate::code_graph::CodeNode::Class(_), "CLASS")
            | (crate::code_graph::CodeNode::Struct(_), "STRUCT")
            | (crate::code_graph::CodeNode::Interface(_), "INTERFACE")
            | (crate::code_graph::CodeNode::Enum(_), "ENUM")
            | (crate::code_graph::CodeNode::TypeAlias(_), "TYPEALIAS")
            | (crate::code_graph::CodeNode::Module(_), "MODULE")
            | (crate::code_graph::CodeNode::Lambda(_), "LAMBDA")
            | (crate::code_graph::CodeNode::Variable(_), "VARIABLE")
            | (crate::code_graph::CodeNode::Constant(_), "CONSTANT")
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

/// Resolve a `ColumnRef`'s sort key for a row: a dotted property access
/// (`n.name`) resolves via [`resolve_property`] (stringified so `Str`
/// and `Int` property values sort uniformly); a bare variable (`n`)
/// sorts by its resolved node id, matching pre-D-05-extension behavior.
/// A sortable `ORDER BY` key. Kept as a small enum (rather than
/// stringifying every property, as this DSL originally did) so numeric
/// properties -- notably X06 core parity's `transitive_loop_depth`/
/// `complexity`/etc, whose values commonly exceed one digit -- sort
/// numerically, not lexicographically (`"20" < "3"` as strings, but
/// `20 > 3` as the integers they are).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum OrderKey {
    // Variant order matters for `Ord`: `Int` sorts before `Str` only
    // when compared across variants, which never happens in practice
    // (a given property always resolves to the same `PropertyValue`
    // variant) -- kept simple rather than reaching for a
    // total-ordering newtype no caller needs.
    Int(i64),
    Str(String),
}

fn order_by_key(column: &ColumnRef, row: &ResultRow, view: &AdjacencyView<'_>) -> OrderKey {
    match &column.property {
        Some(property) => resolve_property(row, view, &column.var, property)
            .map(|value| match value {
                PropertyValue::Str(s) => OrderKey::Str(s),
                PropertyValue::Int(i) => OrderKey::Int(i),
            })
            .unwrap_or(OrderKey::Str(String::new())),
        None => OrderKey::Str(row.get(&column.var).cloned().unwrap_or_default()),
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
        // CLONE-JUSTIFICATION: PropertyValue owns returned values so callers
        // can retain query results after releasing the graph borrow.
        "id" => Some(PropertyValue::Str(node_id.clone())),
        "name" => Some(PropertyValue::Str(node_name(node))),
        "rel_path" | "relpath" | "path" => node_rel_path(node).map(PropertyValue::Str),
        "line" => node_line(node).and_then(|line| match i64::try_from(line) {
            Ok(line) => Some(PropertyValue::Int(line)),
            Err(_) => None,
        }),
        // X06 core parity: Tier A/B complexity properties (see
        // `docs/plans/enforcer-selfhost-plan/refs/
        // x06-baseline-tool-schemas.md` §4.5's property table and
        // `crate::complexity`'s module doc). Booleans surface as `0`/
        // `1` [`PropertyValue::Int`] rather than a `Bool` literal --
        // this DSL's `Literal` has no boolean variant, so a query
        // spells "is true" as `= 1` (matching every other boolean this
        // grammar exposes today, not a complexity-specific choice).
        "complexity" => node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.complexity))),
        "cognitive" => node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.cognitive))),
        "loop_count" => node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.loop_count))),
        "loop_depth" => node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.loop_depth))),
        "param_count" => node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.param_count))),
        "max_access_depth" => {
            node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.max_access_depth)))
        }
        "linear_scan_in_loop" => {
            node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.linear_scan_in_loop)))
        }
        "alloc_in_loop" => {
            node_metrics(node).map(|m| PropertyValue::Int(i64::from(m.alloc_in_loop)))
        }
        "self_recursive" => {
            node_metrics(node).map(|m| PropertyValue::Int(if m.self_recursive { 1 } else { 0 }))
        }
        "recursion_in_loop" => {
            node_metrics(node).map(|m| PropertyValue::Int(if m.recursion_in_loop { 1 } else { 0 }))
        }
        "unguarded_recursion" => node_metrics(node)
            .map(|m| PropertyValue::Int(if m.unguarded_recursion { 1 } else { 0 })),
        "transitive_loop_depth" => node_transitive_metrics(node)
            .map(|m| PropertyValue::Int(i64::from(m.transitive_loop_depth))),
        "recursive" => node_transitive_metrics(node)
            .map(|m| PropertyValue::Int(if m.recursive { 1 } else { 0 })),
        _ => None,
    }
}

fn node_metrics(
    node: &crate::code_graph::CodeNode,
) -> Option<crate::complexity::ComplexityMetrics> {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::Function(s) | CodeNode::Method(s) | CodeNode::Test(s) | CodeNode::Lambda(s) => {
            s.metrics
        }
        _ => None,
    }
}

fn node_transitive_metrics(
    node: &crate::code_graph::CodeNode,
) -> Option<crate::complexity::TransitiveMetrics> {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::Function(s) | CodeNode::Method(s) | CodeNode::Test(s) | CodeNode::Lambda(s) => {
            s.transitive_metrics
        }
        _ => None,
    }
}

fn node_name(node: &crate::code_graph::CodeNode) -> String {
    use crate::code_graph::CodeNode;
    match node {
        // CLONE-JUSTIFICATION: query property values own a snapshot of graph
        // metadata so the caller need not retain the graph borrow.
        CodeNode::File(f) | CodeNode::TextOnly(f) => f.rel_path.clone(),
        CodeNode::Function(s)
        | CodeNode::Type(s)
        | CodeNode::Test(s)
        | CodeNode::Method(s)
        | CodeNode::Class(s)
        | CodeNode::Struct(s)
        | CodeNode::Interface(s)
        | CodeNode::Enum(s)
        | CodeNode::TypeAlias(s)
        | CodeNode::Module(s)
        | CodeNode::Lambda(s)
        | CodeNode::Variable(s)
        | CodeNode::Constant(s) => {
            // CLONE-JUSTIFICATION: query property values own a snapshot of
            // graph metadata after the graph borrow is released.
            s.name.clone()
        }
        // CLONE-JUSTIFICATION: query property values own a snapshot of graph
        // metadata after the graph borrow is released.
        CodeNode::Tombstone(t) => t.rel_path.clone(),
    }
}

fn node_rel_path(node: &crate::code_graph::CodeNode) -> Option<String> {
    use crate::code_graph::CodeNode;
    match node {
        // CLONE-JUSTIFICATION: query property values own a snapshot of graph
        // metadata so the caller need not retain the graph borrow.
        CodeNode::File(f) | CodeNode::TextOnly(f) => Some(f.rel_path.clone()),
        CodeNode::Tombstone(t) => Some(t.rel_path.clone()),
        _ => None,
    }
}

fn node_line(node: &crate::code_graph::CodeNode) -> Option<usize> {
    use crate::code_graph::CodeNode;
    match node {
        CodeNode::Function(s)
        | CodeNode::Type(s)
        | CodeNode::Test(s)
        | CodeNode::Method(s)
        | CodeNode::Class(s)
        | CodeNode::Struct(s)
        | CodeNode::Interface(s)
        | CodeNode::Enum(s)
        | CodeNode::TypeAlias(s)
        | CodeNode::Module(s)
        | CodeNode::Lambda(s)
        | CodeNode::Variable(s)
        | CodeNode::Constant(s) => Some(s.line),
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

fn parse_return_list(cursor: &mut Cursor<'_>) -> Result<(Vec<ColumnRef>, bool), QueryError> {
    let mut vars = Vec::new();
    let mut count = false;
    loop {
        if cursor.peek_keyword("COUNT") {
            cursor.advance();
            cursor.expect_token("(")?;
            let var = cursor.next_token_string()?;
            cursor.expect_token(")")?;
            vars.push(ColumnRef::bare(var));
            count = true;
        } else {
            vars.push(parse_column_ref(cursor)?);
        }
        if cursor.peek_token(",") {
            cursor.advance();
            continue;
        }
        break;
    }
    Ok((vars, count))
}

/// Parse a single column reference: a bare pattern variable (`n`) or a
/// dotted property access on one (`n.name`) -- the same `var.property`
/// form the `WHERE` clause already accepts, reused here so `RETURN` and
/// `ORDER BY` can reference computed/derived properties, not just raw
/// node ids.
fn parse_column_ref(cursor: &mut Cursor<'_>) -> Result<ColumnRef, QueryError> {
    let var = cursor.next_token_string()?;
    if cursor.peek_token(".") {
        cursor.advance();
        let property = cursor.next_token_string()?;
        Ok(ColumnRef {
            var,
            property: Some(property),
        })
    } else {
        Ok(ColumnRef::bare(var))
    }
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
        let Some(c) = chars.get(i).copied() else {
            break;
        };
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            while let Some(current) = chars.get(i).copied() {
                if current == quote {
                    break;
                }
                s.push(current);
                i += 1;
            }
            if i >= chars.len() {
                return Err(QueryError::Parse {
                    pos: i,
                    // ALLOC-JUSTIFICATION: QueryError owns its diagnostic
                    // text after tokenization releases the input buffer.
                    reason: "unterminated string literal".to_string(),
                });
            }
            i += 1;
            tokens.push(Token::Str(s));
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while chars.get(i).is_some_and(|current| current.is_ascii_digit()) {
                i += 1;
            }
            let s: String = chars.get(start..i).into_iter().flatten().collect();
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
            while chars
                .get(i)
                .is_some_and(|current| current.is_alphanumeric() || *current == '_')
            {
                i += 1;
            }
            tokens.push(Token::Word(
                chars.get(start..i).into_iter().flatten().collect(),
            ));
            continue;
        }
        // Multi-char symbols first.
        if c == '.' && chars.get(i + 1) == Some(&'.') {
            tokens.push(Token::Sym("..".to_string()));
            i += 2;
            continue;
        }
        if (c == '!' || c == '<' || c == '>') && chars.get(i + 1) == Some(&'=') {
            let s: String = chars.get(i..i + 2).into_iter().flatten().collect();
            tokens.push(Token::Sym(s));
            i += 2;
            continue;
        }
        if c == '-' && chars.get(i + 1) == Some(&'>') {
            // Emit as two tokens so the parser's explicit `-` then `>`
            // sequence (used for both `-[...]->` arrows) stays uniform.
            // ALLOC-JUSTIFICATION: Token owns punctuation so the parsed AST
            // remains valid after the input buffer is released.
            tokens.push(Token::Sym("-".to_string()));
            tokens.push(Token::Sym(">".to_string()));
            i += 2;
            continue;
        }
        // ALLOC-JUSTIFICATION: Token owns punctuation so the parsed AST
        // remains valid after the input buffer is released.
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
                // CLONE-JUSTIFICATION: the parsed AST owns its token text
                // after Cursor advances beyond this borrowed token stream.
                let w = w.clone();
                self.advance();
                Ok(w)
            }
            Some(Token::Int(n)) => {
                // ALLOC-JUSTIFICATION: numeric tokens become owned AST text
                // after Cursor advances beyond this borrowed token stream.
                let s = n.to_string();
                self.advance();
                Ok(s)
            }
            Some(Token::Sym(s)) => {
                // CLONE-JUSTIFICATION: the parsed AST owns its token text
                // after Cursor advances beyond this borrowed token stream.
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
                // CLONE-JUSTIFICATION: Literal owns its text after Cursor
                // advances beyond this borrowed token stream.
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
