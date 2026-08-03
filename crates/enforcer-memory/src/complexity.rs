//! X06 core parity: per-function complexity signals.
//!
//! Mirrors the C baseline's two-tier complexity pipeline
//! (`codebase-memory-mcp/src/pipeline/pass_parallel.c::build_def_props`
//! for Tier A, `pass_complexity.c::cbm_pipeline_pass_complexity` for
//! Tier B â€” see `docs/plans/enforcer-selfhost-plan/refs/
//! x06-baseline-tool-schemas.md` Â§4.5 for the authoritative property
//! table this module reproduces):
//!
//! - **Tier A** ([`ComplexityMetrics::compute`]): local, per-function
//!   structural metrics computed directly from one function/method's
//!   AST subtree -- cyclomatic complexity, cognitive complexity, loop
//!   count/depth, parameter count, max chained-access depth, and four
//!   boolean/count "bottleneck candidate" signals (linear scans and
//!   allocations inside loops, self-recursion, recursion inside a
//!   loop, unguarded recursion).
//! - **Tier B** ([`propagate_transitive_loop_depth`]): an
//!   interprocedural pass over the call graph that propagates each
//!   function's `loop_depth` along `CALLS` edges to estimate a
//!   worst-case *transitive* nested-loop degree, exactly like the C
//!   baseline's memoized DFS (`tld_dfs`) -- cycles are broken and
//!   folded into a `recursive` flag rather than causing non-termination.
//!
//! # Language-neutral design
//!
//! Every metric is computed over a [`NodeKindTable`] of tree-sitter
//! node-kind strings supplied per language, not over per-language
//! branching inside the walk itself -- adding wave-B language X means
//! adding one `NodeKindTable::for_x()` constructor, not touching this
//! module's walk logic. [`NodeKindTable::rust`], [`NodeKindTable::typescript_javascript`],
//! and [`NodeKindTable::python`] are the wave-A tables.

use enforcer_domain::memory_types::{
    ComplexityCallGraph, ComplexityCallGraphNode, ComplexityMeasure, ComplexityNodeId,
    ComplexityNodeKindPresence, ComplexityPropagation, ComplexitySourceBytes, ComplexitySourceText,
    ComplexitySymbolLocation, ComplexityTransitiveMetrics, GraphSourceLine, MemoryAstFieldName,
    MemoryAstNodeKind, MemoryAstNodeKindSet, ParsedCallee, ParsedSymbolName,
};
use enforcer_syntax::parsers::grammar_for_complexity;
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

/// Per-language tree-sitter node-kind classification this module's AST
/// walk needs. Every field is a set of node `.kind()` strings (matched
/// by exact string equality, no regex) -- kept as a flat table rather
/// than a trait so a new language is "list the kind strings" with no
/// new virtual dispatch.
#[derive(Debug, Clone, Default)]
pub struct NodeKindTable {
    /// Decision-point node kinds that each add 1 to cyclomatic
    /// complexity (`if`, `else if`/`elif`, `match`/`switch` arms,
    /// `while`, `for`, `catch`/`except`, `&&`/`||`-style boolean
    /// binary operators, ternary/conditional expressions).
    decision_points: MemoryAstNodeKindSet,
    /// Loop node kinds (`for`, `while`, `loop`, comprehension-with-`for`).
    loops: MemoryAstNodeKindSet,
    /// Nesting-increasing node kinds for cognitive complexity (loops +
    /// conditionals + `catch`/`except` + nested closures/functions).
    nesting: MemoryAstNodeKindSet,
    /// The function/method definition node kind(s) themselves (used to
    /// recognize a nested function boundary while walking, so a
    /// closure's own decision points are not double counted into the
    /// enclosing function AND the metric still detects nesting).
    function_defs: MemoryAstNodeKindSet,
    /// Call-expression node kind(s).
    calls: MemoryAstNodeKindSet,
    /// The field name (on a call-expression node) holding the callee.
    call_function_field: MemoryAstFieldName,
    /// Member/property/field access expression node kind(s), used for
    /// `max_access_depth` (chained `a.b.c.d`).
    member_access: MemoryAstNodeKindSet,
    /// The field name (on a member-access node) holding the base being
    /// accessed (its "left"/"object" side) -- walking this field
    /// repeatedly measures chain depth.
    member_access_object_field: MemoryAstFieldName,
    /// Parameter-list node kind(s) for a function/method definition.
    parameters: MemoryAstNodeKindSet,
    /// The field name on a function/method definition node holding its
    /// parameter list.
    parameters_field: MemoryAstFieldName,
    /// The field name on a function/method definition node holding its
    /// own name (used to detect self-calls for recursion).
    name_field: MemoryAstFieldName,
    /// The field name on a function/method definition node holding its
    /// body block.
    body_field: MemoryAstFieldName,
    /// If-statement/expression node kind(s) (used separately from
    /// `decision_points` to walk the `else`/`elif` chain for cognitive
    /// complexity's "each additional branch, not just the first, adds
    /// nesting-weighted cost" rule).
    if_statements: MemoryAstNodeKindSet,
}

macro_rules! node_kind_table {
    (
        decision_points: $decision_points:expr,
        loops: $loops:expr,
        nesting: $nesting:expr,
        function_defs: $function_defs:expr,
        calls: $calls:expr,
        call_function_field: $call_function_field:expr,
        member_access: $member_access:expr,
        member_access_object_field: $member_access_object_field:expr,
        parameters: $parameters:expr,
        parameters_field: $parameters_field:expr,
        name_field: $name_field:expr,
        body_field: $body_field:expr,
        if_statements: $if_statements:expr $(,)?
    ) => {
        NodeKindTable {
            decision_points: MemoryAstNodeKindSet::from_static($decision_points),
            loops: MemoryAstNodeKindSet::from_static($loops),
            nesting: MemoryAstNodeKindSet::from_static($nesting),
            function_defs: MemoryAstNodeKindSet::from_static($function_defs),
            calls: MemoryAstNodeKindSet::from_static($calls),
            call_function_field: MemoryAstFieldName::from_static($call_function_field),
            member_access: MemoryAstNodeKindSet::from_static($member_access),
            member_access_object_field: MemoryAstFieldName::from_static(
                $member_access_object_field,
            ),
            parameters: MemoryAstNodeKindSet::from_static($parameters),
            parameters_field: MemoryAstFieldName::from_static($parameters_field),
            name_field: MemoryAstFieldName::from_static($name_field),
            body_field: MemoryAstFieldName::from_static($body_field),
            if_statements: MemoryAstNodeKindSet::from_static($if_statements),
        }
    };
}

impl NodeKindTable {
    /// Whether this language table recognizes any loop node kinds.
    pub fn has_loop_kinds(&self) -> ComplexityNodeKindPresence {
        (!self.loops.is_empty()).into()
    }

    /// Whether this language table recognizes any decision-point node kinds.
    pub fn has_decision_point_kinds(&self) -> ComplexityNodeKindPresence {
        (!self.decision_points.is_empty()).into()
    }

    /// Return the Rust AST node-kind classification table.
    pub fn rust() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_expression",
                "if_let_expression",
                "match_arm",
                "while_expression",
                "while_let_expression",
                "loop_expression",
                "for_expression",
                "&&",
                "||",
            ],
            loops: &[
                "while_expression",
                "while_let_expression",
                "loop_expression",
                "for_expression",
            ],
            nesting: &[
                "if_expression",
                "if_let_expression",
                "match_expression",
                "while_expression",
                "while_let_expression",
                "loop_expression",
                "for_expression",
                "closure_expression",
            ],
            function_defs: &["function_item", "closure_expression"],
            calls: &["call_expression", "method_call_expression"],
            call_function_field: "function",
            member_access: &["field_expression"],
            member_access_object_field: "value",
            parameters: &["parameters"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_expression", "if_let_expression"],
        }
    }

    /// Return the TypeScript and JavaScript AST node-kind classification table.
    pub fn typescript_javascript() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "switch_case",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_in_statement",
                "catch_clause",
                "&&",
                "||",
                "ternary_expression",
            ],
            loops: &[
                "while_statement",
                "do_statement",
                "for_statement",
                "for_in_statement",
            ],
            nesting: &[
                "if_statement",
                "switch_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_in_statement",
                "catch_clause",
                "function_expression",
                "arrow_function",
            ],
            function_defs: &[
                "function_declaration",
                "function_expression",
                "arrow_function",
                "method_definition",
            ],
            calls: &["call_expression"],
            call_function_field: "function",
            member_access: &["member_expression"],
            member_access_object_field: "object",
            parameters: &["formal_parameters"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// Return the Python AST node-kind classification table.
    pub fn python() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "elif_clause",
                "while_statement",
                "for_statement",
                "except_clause",
                "boolean_operator",
                "conditional_expression",
            ],
            loops: &["while_statement", "for_statement"],
            nesting: &[
                "if_statement",
                "while_statement",
                "for_statement",
                "except_clause",
                "lambda",
            ],
            function_defs: &["function_definition", "lambda"],
            calls: &["call"],
            call_function_field: "function",
            member_access: &["attribute"],
            member_access_object_field: "object",
            parameters: &["parameters"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// Return the Go AST node-kind classification table.
    pub fn go() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "expression_case",
                "type_case",
                "for_statement",
                "&&",
                "||",
            ],
            loops: &["for_statement"],
            nesting: &[
                "if_statement",
                "expression_switch_statement",
                "type_switch_statement",
                "for_statement",
                "func_literal",
            ],
            function_defs: &["function_declaration", "method_declaration", "func_literal"],
            calls: &["call_expression"],
            call_function_field: "function",
            member_access: &["selector_expression"],
            member_access_object_field: "operand",
            parameters: &["parameter_list"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// Return the Java AST node-kind classification table.
    pub fn java() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "switch_label",
                "while_statement",
                "do_statement",
                // Java's for-each (`for (T x : xs)`) shares the plain
                // `for_statement` node kind with the classic C-style
                // for-loop -- there is no separate `for_each_statement`
                // kind in this grammar.
                "for_statement",
                "catch_clause",
                "&&",
                "||",
                "ternary_expression",
            ],
            loops: &["while_statement", "do_statement", "for_statement"],
            nesting: &[
                "if_statement",
                "switch_expression",
                "while_statement",
                "do_statement",
                "for_statement",
                "catch_clause",
                "lambda_expression",
            ],
            function_defs: &[
                "method_declaration",
                "constructor_declaration",
                "lambda_expression",
            ],
            calls: &["method_invocation"],
            call_function_field: "name",
            member_access: &["field_access"],
            member_access_object_field: "object",
            parameters: &["formal_parameters"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// C: `switch`/`case_statement` (matching the workpack's "decisions
    /// incl. switch cases" instruction -- each `case`/`default` arm adds
    /// 1, not the `switch_statement` itself), `for`/`while`/`do` loops,
    /// `->`/`.`-chained [`field_expression`] access, and pointer/`new`-
    /// style allocation callees (`malloc`/`calloc`/`realloc`, matched
    /// via [`ALLOC_CALLEES`] plus C's own idioms below).
    pub fn c() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "&&",
                "||",
            ],
            loops: &["while_statement", "do_statement", "for_statement"],
            nesting: &[
                "if_statement",
                "switch_statement",
                "while_statement",
                "do_statement",
                "for_statement",
            ],
            function_defs: &["function_definition"],
            calls: &["call_expression"],
            call_function_field: "function",
            member_access: &["field_expression"],
            member_access_object_field: "argument",
            parameters: &["parameter_list"],
            parameters_field: "parameters",
            name_field: "declarator",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// C++: `languages::cpp`'s superset of `c()` -- adds `switch_statement`/
    /// `case_statement`, range-`for`, `catch_clause`, and lambda
    /// expressions to the nesting/function-boundary sets (matching the
    /// workpack's "decisions incl. switch cases, loops, param lists,
    /// field/arrow access chains, malloc/new/push_back-style allocs"
    /// instruction -- `new`/`push_back` ride the shared
    /// [`ALLOC_CALLEES`] table, matched on the callee's last path
    /// segment the same way `find`/`push`/etc already are for every
    /// other language).
    pub fn cpp() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_range_loop",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            loops: &[
                "while_statement",
                "do_statement",
                "for_statement",
                "for_range_loop",
            ],
            nesting: &[
                "if_statement",
                "switch_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "for_range_loop",
                "catch_clause",
                "lambda_expression",
            ],
            function_defs: &["function_definition", "lambda_expression"],
            calls: &["call_expression"],
            call_function_field: "function",
            member_access: &["field_expression"],
            member_access_object_field: "argument",
            parameters: &["parameter_list"],
            parameters_field: "parameters",
            name_field: "declarator",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// C#: `switch_expression_arm`/`switch_section` count as decision
    /// points (one per arm/case, matching `switch_case`'s treatment
    /// elsewhere), `foreach_statement` joins `for`/`while`/`do` as a
    /// loop, and lambdas/local functions extend the nesting/function-
    /// boundary sets the same way closures do for every other language.
    pub fn csharp() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "switch_section",
                "switch_expression_arm",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            loops: &[
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
            ],
            nesting: &[
                "if_statement",
                "switch_statement",
                "switch_expression",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "lambda_expression",
                "anonymous_method_expression",
                "local_function_statement",
            ],
            function_defs: &[
                "method_declaration",
                "constructor_declaration",
                "lambda_expression",
                "anonymous_method_expression",
                "local_function_statement",
            ],
            calls: &["invocation_expression"],
            call_function_field: "function",
            member_access: &["member_access_expression"],
            member_access_object_field: "expression",
            parameters: &["parameter_list"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }

    /// PHP: `switch` `case_statement`s, `foreach`/`for`/`while`/`do`
    /// loops, `->`/`?->`-chained [`member_access_expression`] access,
    /// and closures/arrow-functions extend the nesting/function-
    /// boundary sets.
    pub fn php() -> Self {
        node_kind_table! {
            decision_points: &[
                "if_statement",
                "case_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "&&",
                "||",
                "conditional_expression",
            ],
            loops: &[
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
            ],
            nesting: &[
                "if_statement",
                "switch_statement",
                "while_statement",
                "do_statement",
                "for_statement",
                "foreach_statement",
                "catch_clause",
                "anonymous_function",
                "arrow_function",
            ],
            function_defs: &[
                "function_definition",
                "method_declaration",
                "anonymous_function",
                "arrow_function",
            ],
            calls: &["function_call_expression", "member_call_expression"],
            call_function_field: "function",
            member_access: &["member_access_expression"],
            member_access_object_field: "object",
            parameters: &["formal_parameters"],
            parameters_field: "parameters",
            name_field: "name",
            body_field: "body",
            if_statements: &["if_statement"],
        }
    }
}

/// Linear-scan-style callee names ("bottleneck candidate" signal --
/// see the module doc's Tier A table): calling any of these inside a
/// loop is an O(n) scan per iteration, i.e. the hidden O(n^2) that a
/// purely syntactic `loop_depth` count misses. Matched against the
/// callee's *last* path segment (so `self.items.iter().find` and
/// `list.find(...)` both match on `find`), case-sensitive.
const LINEAR_SCAN_CALLEES: &[&str] = &[
    "find", "contains", "indexOf", "index_of", "includes", "some", "any", "filter",
];

/// Allocation/append-style callee names for `alloc_in_loop`. Matched
/// the same way as [`LINEAR_SCAN_CALLEES`].
const ALLOC_CALLEES: &[&str] = &[
    "new",
    "push",
    "append",
    "extend",
    "format",
    "to_string",
    "clone",
    "insert",
    "collect",
    // C/C++ (lane x06-b2-ccpp): heap allocation and vector-append
    // idioms -- `malloc`/`calloc`/`realloc` (C), `push_back`/
    // `emplace_back` (C++ std::vector-style containers).
    "malloc",
    "calloc",
    "realloc",
    "push_back",
    "emplace_back",
];

/// Tier A metrics for one function/method, additive on [`crate::code_graph::SymbolNode`]
/// (existing callers/tests that never construct this see no change --
/// see `code_graph.rs` for the `Option<ComplexityMetrics>` field this
/// type populates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ComplexityMetrics {
    pub complexity: ComplexityMeasure,
    pub cognitive: ComplexityMeasure,
    pub loop_count: ComplexityMeasure,
    pub loop_depth: ComplexityMeasure,
    pub param_count: ComplexityMeasure,
    pub max_access_depth: ComplexityMeasure,
    pub linear_scan_in_loop: ComplexityMeasure,
    pub alloc_in_loop: ComplexityMeasure,
    /// Tier A "self_recursive" seed in the C baseline: a *direct*
    /// self-call only. [`TransitiveMetrics::recursive`] is the
    /// separate, OR'd-in Tier B property -- see this module's doc
    /// comment and the baseline doc's "naming hazard" note. Do not
    /// collapse the two.
    pub self_recursive: ComplexitySignal,
    pub recursion_in_loop: ComplexitySignal,
    pub unguarded_recursion: ComplexitySignal,
}

/// Tier B metrics, populated by [`propagate_transitive_loop_depth`]
/// after the whole repository's call graph is known -- kept as a
/// separate type from [`ComplexityMetrics`] because it is computed in
/// a distinct pass (interprocedural, not per-function) and, per the
/// baseline's naming-hazard note, `recursive` here is semantically
/// different from `self_recursive` above (self-recursion OR a
/// call-graph cycle via mutual recursion).
/// Compute Tier A metrics for one function/method definition node.
///
/// `function_name` is the symbol's own name (used to recognize direct
/// self-calls for `self_recursive`/`recursion_in_loop`/
/// `unguarded_recursion` -- a call whose callee text's last path
/// segment equals this name). `src` is the whole file's source bytes
/// (tree-sitter nodes are byte-range views into it).
pub fn compute<'source>(
    def_node: Node<'_>,
    function_name: impl Into<ParsedSymbolName>,
    src: impl Into<ComplexitySourceBytes<'source>>,
    table: &NodeKindTable,
) -> ComplexityMetrics {
    let function_name = function_name.into();
    let function_name = function_name.as_str();
    let src = src.into();
    let src = src.as_bytes();
    let body = def_node
        .child_by_field_name(table.body_field.as_str())
        .unwrap_or(def_node);

    let mut metrics = ComplexityMetrics {
        complexity: ComplexityMeasure::BASELINE,
        param_count: param_count(def_node, table),
        ..ComplexityMetrics::default()
    };

    let mut ctx = WalkCtx {
        table,
        src,
        function_name,
        metrics: &mut metrics,
    };
    walk_body(
        body,
        &mut ctx,
        ComplexityMeasure::ZERO,
        ComplexityMeasure::ZERO,
        NestedFunctionState::Root,
    );
    metrics.max_access_depth = max_access_depth(body, table);
    metrics
}

/// Find the function/method definition node named `name` starting at
/// 1-based `line` (matching [`crate::parsers::SymbolRef::line`]/
/// [`crate::code_graph::SymbolNode::line`]'s convention), searching
/// `root`'s subtree. Intended as the integration seam between this
/// module and a caller that only has a `(name, line)` pair from a
/// [`crate::parsers::ParsedFile`] extraction pass, not a live AST
/// handle -- see `complexity.rs`'s module doc and `code_graph.rs`'s
/// `insert_file_and_chunks` for how a caller re-locates the node to
/// call [`compute`] on. Returns `None` if no `function_defs`-kind node
/// with that name/line exists (e.g. the name/line pair is stale, or
/// the language table's `name_field` doesn't apply to this node
/// shape).
pub fn find_definition_node<'tree, 'source>(
    root: Node<'tree>,
    name: impl Into<ParsedSymbolName>,
    line: impl Into<GraphSourceLine>,
    src: impl Into<ComplexitySourceBytes<'source>>,
    table: &NodeKindTable,
) -> Option<Node<'tree>> {
    let name = name.into();
    let name = name.as_str();
    let line = line.into().get();
    let src = src.into();
    let src = src.as_bytes();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if table
            .function_defs
            .has_node_kind(MemoryAstNodeKind::from(node.kind()))
            && node.start_position().row + 1 == line
        {
            if let Some(name_node) = node.child_by_field_name(table.name_field.as_str()) {
                // Exact-field text match (every wave-A/B language table
                // so far: `name_field` names a leaf identifier node
                // directly, e.g. Rust/TS/Python/Go/Java's own `name`
                // field). Kept as the primary, unchanged path so no
                // existing language's behavior shifts by even one
                // node.
                if matches!(name_node.utf8_text(src), Ok(node_name) if node_name == name) {
                    return Some(node);
                }
                // C/C++ fallback: `name_field` (`"declarator"`) names a
                // *nested* declarator subtree (`function_declarator`,
                // possibly wrapped in `pointer_declarator`s for a
                // pointer-returning function) whose own `.utf8_text()`
                // is the whole declarator (`"foo(int x)"`), not the
                // bare name -- so this walks that subtree's innermost
                // identifier instead of comparing the field's own text.
                if let Some(inner_name) = innermost_identifier_text(name_node, src.into()) {
                    if inner_name.as_str() == name {
                        return Some(node);
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    None
}

/// Walk a declarator subtree (`function_declarator` /
/// `pointer_declarator` / `parenthesized_declarator` / `qualified_identifier`,
/// the C/C++ grammar shapes [`find_definition_node`]'s fallback needs)
/// down to its innermost bare identifier's text. Returns `None` for any
/// node shape this does not recognize (every other language's
/// `name_field` already resolves directly and never reaches this
/// fallback at all).
fn innermost_identifier_text<'a>(
    node: Node<'_>,
    src: ComplexitySourceBytes<'a>,
) -> Option<ComplexitySourceText<'a>> {
    let src = src.as_bytes();
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "destructor_name"
        | "operator_name" => match node.utf8_text(src) {
            Ok(text) => Some(text.into()),
            Err(_) => None,
        },
        "function_declarator"
        | "pointer_declarator"
        | "reference_declarator"
        | "parenthesized_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| innermost_identifier_text(inner, src.into())),
        "qualified_identifier" => node
            .child_by_field_name("name")
            .and_then(|inner| innermost_identifier_text(inner, src.into())),
        _ => None,
    }
}

fn param_count(def_node: Node<'_>, table: &NodeKindTable) -> ComplexityMeasure {
    let direct = def_node.child_by_field_name(table.parameters_field.as_str());
    // C/C++ fallback: `function_definition`'s own fields are
    // `type`/`declarator`/`body` -- the parameter list lives one level
    // deeper, on the `declarator` (`function_declarator`)'s own
    // `parameters` field, not directly on `def_node`. Every other
    // language's table already resolves `parameters_field` directly on
    // `def_node` (this `direct` lookup succeeds and the fallback is
    // never reached for them).
    let params = match direct {
        Some(params) if table.parameters.has_node_kind(params.kind().into()) => params,
        _ => {
            let Some(nested) = def_node
                .child_by_field_name(table.name_field.as_str())
                .and_then(|declarator| {
                    declarator.child_by_field_name(table.parameters_field.as_str())
                })
            else {
                return ComplexityMeasure::ZERO;
            };
            nested
        }
    };
    if !table.parameters.has_node_kind(params.kind().into()) {
        return ComplexityMeasure::ZERO;
    }
    let mut count = ComplexityMeasure::ZERO;
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        let kind = child.kind();
        // Skip punctuation/comment tokens; every language's
        // parameter-list grammar emits the parameter as either a
        // single named node or an "identifier"-shaped leaf -- both
        // are non-punctuation kinds, so filtering `(`, `)`, `,`
        // leaves exactly the parameters.
        if kind == "(" || kind == ")" || kind == "," || kind.is_empty() {
            continue;
        }
        if child.is_extra() {
            continue;
        }
        count += 1;
    }
    count
}

struct WalkCtx<'a, 'src> {
    table: &'a NodeKindTable,
    // BRAND-INVARIANT: this slice is the exact source buffer used by the
    // tree-sitter nodes in the current walk and remains borrowed for its
    // entire lifetime.
    src: &'src [u8],
    // BRAND-INVARIANT: this name is the stable symbol label selected by the
    // caller for the metrics being accumulated.
    function_name: &'a str,
    metrics: &'a mut ComplexityMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NestedFunctionState {
    Root,
    Nested,
}

/// Walk one function's body subtree, accumulating Tier A metrics.
/// `loop_depth` is the current nesting depth of enclosing loops (used
/// for `loop_depth`/`linear_scan_in_loop`/`alloc_in_loop`/
/// `recursion_in_loop`); `cognitive_nesting` is the current cognitive
/// nesting level (loops + conditionals, per the standard cognitive
/// complexity formulation: nesting constructs cost `1 + nesting_level`
/// each, not a flat `1`); `in_nested_fn` stops recursion into a nested
/// closure/function's own body (that subtree gets its own `compute`
/// call as its own symbol; double-walking it here would double count).
fn walk_body(
    node: Node<'_>,
    ctx: &mut WalkCtx<'_, '_>,
    loop_depth: ComplexityMeasure,
    cognitive_nesting: ComplexityMeasure,
    in_nested_fn: NestedFunctionState,
) {
    let kind = node.kind();

    if matches!(in_nested_fn, NestedFunctionState::Nested) {
        // Still need to descend to find further nested boundaries, but
        // stop attributing metrics to the outer function.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_body(
                child,
                ctx,
                loop_depth,
                cognitive_nesting,
                NestedFunctionState::Nested,
            );
        }
        return;
    }

    let is_loop = ctx.table.loops.has_node_kind(kind.into());
    let is_decision = ctx.table.decision_points.has_node_kind(kind.into());
    let is_nesting = ctx.table.nesting.has_node_kind(kind.into());

    if is_decision {
        ctx.metrics.complexity += 1;
    }
    if is_nesting {
        ctx.metrics.cognitive += 1 + cognitive_nesting.get();
    }

    let child_loop_depth = if is_loop {
        ctx.metrics.loop_count += 1;
        let new_depth = ComplexityMeasure::from(loop_depth.get().saturating_add(1));
        if new_depth.get() > ctx.metrics.loop_depth.get() {
            ctx.metrics.loop_depth = new_depth;
        }
        new_depth
    } else {
        loop_depth
    };
    let child_cognitive_nesting = if is_nesting {
        ComplexityMeasure::from(cognitive_nesting.get().saturating_add(1))
    } else {
        cognitive_nesting
    };

    if ctx.table.calls.has_node_kind(kind.into()) {
        inspect_call(node, ctx, loop_depth);
    }

    let entering_nested_fn = if ctx.table.function_defs.has_node_kind(kind.into()) {
        NestedFunctionState::Nested
    } else {
        NestedFunctionState::Root
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_body(
            child,
            ctx,
            child_loop_depth,
            child_cognitive_nesting,
            entering_nested_fn,
        );
    }
}

fn inspect_call(call_node: Node<'_>, ctx: &mut WalkCtx<'_, '_>, loop_depth: ComplexityMeasure) {
    let Some(function_node) = call_node.child_by_field_name(ctx.table.call_function_field.as_str())
    else {
        return;
    };
    let Ok(callee_text) = function_node.utf8_text(ctx.src) else {
        return;
    };
    let last_segment = last_path_segment(&ParsedCallee::from(callee_text));

    if last_segment.as_str() == ctx.function_name {
        ctx.metrics.self_recursive = ComplexitySignal::Present;
        if loop_depth.get() > 0 {
            ctx.metrics.recursion_in_loop = ComplexitySignal::Present;
        }
        if !bool::from(recursion_is_guarded(call_node, ctx.table)) {
            ctx.metrics.unguarded_recursion = ComplexitySignal::Present;
        }
    }

    if loop_depth.get() > 0 {
        if LINEAR_SCAN_CALLEES.contains(&last_segment.as_str()) {
            ctx.metrics.linear_scan_in_loop += 1;
        }
        if ALLOC_CALLEES.contains(&last_segment.as_str()) {
            ctx.metrics.alloc_in_loop += 1;
        }
    }
}

/// The last `.`/`::`-separated segment of a callee expression's text,
/// e.g. `self.items.iter().find` -> `find`, `helper` -> `helper`.
fn last_path_segment(callee_text: &ParsedCallee) -> ParsedSymbolName {
    callee_text
        .as_str()
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(callee_text.as_str())
        .trim_end_matches(['(', ')'])
        .into()
}

/// Whether a self-recursive call is reachable only through a
/// conditionally-guarded path, i.e. some ancestor between the call and
/// the function body is an if-statement/conditional (a base case
/// exists). Walking ancestors rather than requiring a literal `return`
/// keeps this check simple and matches the baseline's "no
/// conditionally-guarded base case" wording -- an unconditional
/// self-call at the top level of the function body (no enclosing `if`)
/// is unguarded infinite recursion by construction.
fn recursion_is_guarded(call_node: Node<'_>, table: &NodeKindTable) -> ComplexityNodeKindPresence {
    let mut current = call_node.parent();
    while let Some(node) = current {
        if table.if_statements.has_node_kind(node.kind().into())
            || table.decision_points.has_node_kind(node.kind().into())
        {
            return true.into();
        }
        if table.function_defs.has_node_kind(node.kind().into()) {
            break; // reached the enclosing function boundary; stop.
        }
        current = node.parent();
    }
    false.into()
}

/// Max chained member/property access depth (`a.b.c.d` -> 4) anywhere
/// in the subtree. Walks each member-access node's "object" side
/// repeatedly to measure one chain's depth, and takes the max across
/// every chain found in the subtree (a function with several unrelated
/// short chains and one long one is scored by the long one).
fn max_access_depth(node: Node<'_>, table: &NodeKindTable) -> ComplexityMeasure {
    let mut best = ComplexityMeasure::ZERO;
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        // `compute` reports metrics for one definition only.  `walk_body`
        // already treats nested closures/functions as a boundary, so access
        // depth must do the same: a chain inside a closure belongs to that
        // closure's symbol, not its enclosing function.
        if table.function_defs.has_node_kind(current.kind().into()) && current.id() != node.id() {
            continue;
        }
        if table.member_access.has_node_kind(current.kind().into()) {
            let depth = chain_depth(current, table);
            if depth.get() > best.get() {
                best = depth;
            }
            // Do not descend further into an already-measured chain's
            // own object side (chain_depth already walked it) -- but
            // do still walk sibling/non-object children (e.g. call
            // arguments) for other chains.
            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                let is_object_side = current
                    .child_by_field_name(table.member_access_object_field.as_str())
                    .map(|o| o.id() == child.id())
                    .unwrap_or(false);
                if !is_object_side {
                    stack.push(child);
                }
            }
        } else {
            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                stack.push(child);
            }
        }
    }
    best
}

fn chain_depth(node: Node<'_>, table: &NodeKindTable) -> ComplexityMeasure {
    let mut depth = ComplexityMeasure::BASELINE;
    let mut current = node;
    while table.member_access.has_node_kind(current.kind().into()) {
        depth += 1;
        match current.child_by_field_name(table.member_access_object_field.as_str()) {
            Some(next) => current = next,
            None => break,
        }
    }
    depth
}

/// One node in the call graph [`propagate_transitive_loop_depth`]
/// walks: identified by `id` (matching [`crate::code_graph::SymbolNode::id`]),
/// carrying its own Tier A `loop_depth` and `self_recursive` seed, and
/// the set of callee ids it calls (already resolved to graph node ids
/// by the caller -- this module does not do call-target resolution).
/// Tier B: propagate `loop_depth` along `CALLS` edges into a
/// worst-case transitive nested-loop estimate, mirroring the C
/// baseline's memoized DFS (`pass_complexity.c::tld_dfs`) --
/// `tld(id) = loop_depth(id) + max(tld(callee) for callee in calls(id))`,
/// with call-graph cycles (including indirect/mutual recursion)
/// detected via an in-progress marker and folded into `recursive`
/// rather than causing unbounded recursion. A depth cap
/// ([`MAX_PROPAGATION_DEPTH`], matching the baseline's
/// `CBM_TLD_MAX_DEPTH = 256`) is a second, belt-and-suspenders guard
/// against pathological call graphs.
///
/// Returns a map from node id to its [`TransitiveMetrics`]. Every id
/// present in `nodes` is guaranteed a key in the result (bounded
/// fixpoint: this function always terminates and always returns full
/// coverage, even for a fully-cyclic call graph).
pub fn propagate_transitive_loop_depth(graph: ComplexityCallGraph<'_>) -> ComplexityPropagation {
    let nodes = graph.nodes();
    const MAX_PROPAGATION_DEPTH: u32 = 256;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Unvisited,
        InProgress,
        Done,
    }

    // Bundles the DFS's shared mutable maps into one value so the
    // recursive walk takes one context argument instead of four
    // separate `&mut HashMap` parameters (clippy's `too_many_arguments`
    // -- this is the same memoization the C baseline's `tld_dfs` does
    // with raw arrays, just keyed by id instead of by array index).
    struct DfsCtx<'a> {
        by_id: HashMap<ComplexityNodeId, &'a ComplexityCallGraphNode>,
        state: HashMap<ComplexityNodeId, State>,
        // BRAND-INVARIANT: each entry is the memoized transitive loop depth
        // for a validated complexity node id.
        tld: HashMap<ComplexityNodeId, ComplexityMeasure>,
        // BRAND-INVARIANT: each entry records whether the corresponding
        // validated complexity node participates in a recursive cycle.
        recursive: HashMap<ComplexityNodeId, bool>,
        active_path: Vec<ComplexityNodeId>,
        // BRAND-INVARIANT: this is the maximum DFS depth observed for the
        // validated call graph, including zero for an empty traversal.
        max_depth: ComplexityMeasure,
    }

    fn dfs(
        id: &ComplexityNodeId,
        depth: ComplexityMeasure,
        ctx: &mut DfsCtx<'_>,
    ) -> ComplexityMeasure {
        let Some(node) = ctx.by_id.get(id).copied() else {
            return ComplexityMeasure::ZERO;
        };
        match ctx.state.get(id).copied().unwrap_or(State::Unvisited) {
            State::Done => return ctx.tld.get(id).copied().unwrap_or(ComplexityMeasure::ZERO),
            State::InProgress => {
                // Back edge -> call-graph cycle: fold into `recursive`,
                // contribute zero additional depth from this branch
                // (matching the baseline's tld_dfs early return). Mark
                // every participant on the active path, not only the
                // back-edge target: each function in a mutual-recursion
                // cycle is recursive by definition.
                if let Some(cycle_start) = ctx.active_path.iter().position(|entry| entry == id) {
                    for participant in ctx.active_path.iter().skip(cycle_start) {
                        if let Some(recursive) = ctx.recursive.get_mut(participant) {
                            *recursive = true;
                        }
                    }
                }
                return ComplexityMeasure::ZERO;
            }
            State::Unvisited => {}
        }
        if depth.get() > ctx.max_depth.get() {
            return node.loop_depth;
        }
        ctx.state.insert(id.retained(), State::InProgress);
        ctx.active_path.push(id.retained());
        let mut best = ComplexityMeasure::ZERO;
        for callee in &node.callees {
            if callee == id {
                ctx.recursive.insert(id.retained(), true);
                continue;
            }
            let callee_tld = dfs(
                callee,
                ComplexityMeasure::from(depth.get().saturating_add(1)),
                ctx,
            );
            if callee_tld.get() > best.get() {
                best = callee_tld;
            }
        }
        // Transitive depth is a bounded risk estimate. Saturate instead of
        // panicking in debug or wrapping in release when a caller supplies
        // an extreme graph metric.
        let value = ComplexityMeasure::from(node.loop_depth.get().saturating_add(best.get()));
        ctx.active_path.pop();
        ctx.tld.insert(id.retained(), value);
        ctx.state.insert(id.retained(), State::Done);
        value
    }

    let by_id: HashMap<ComplexityNodeId, &ComplexityCallGraphNode> =
        nodes.iter().map(|n| (n.id.retained(), n)).collect();
    let state: HashMap<ComplexityNodeId, State> = nodes
        .iter()
        .map(|n| (n.id.retained(), State::Unvisited))
        .collect();
    let recursive: HashMap<ComplexityNodeId, bool> = nodes
        .iter()
        .map(|n| (n.id.retained(), n.self_recursive.is_present()))
        .collect();
    let mut ctx = DfsCtx {
        by_id,
        state,
        tld: HashMap::new(),
        recursive,
        active_path: Vec::new(),
        max_depth: ComplexityMeasure::from(MAX_PROPAGATION_DEPTH),
    };

    for node in nodes {
        if ctx.state.get(&node.id).copied() != Some(State::Done) {
            dfs(&node.id, ComplexityMeasure::ZERO, &mut ctx);
        }
    }
    let tld = ctx.tld;
    let recursive = ctx.recursive;

    nodes
        .iter()
        .map(|n| {
            let metrics = ComplexityTransitiveMetrics {
                transitive_loop_depth: tld.get(&n.id).copied().unwrap_or(n.loop_depth),
                recursive: recursive.get(&n.id).copied().unwrap_or(false).into(),
            };
            (n.id.retained(), metrics)
        })
        .collect()
}

use crate::owned_boundary::Retained;
use enforcer_domain::memory_types::{ComplexityLanguage, ComplexitySignal};

fn node_kind_table(language: ComplexityLanguage) -> NodeKindTable {
    match language {
        ComplexityLanguage::Rust => NodeKindTable::rust(),
        ComplexityLanguage::TypeScriptOrJavaScript => NodeKindTable::typescript_javascript(),
        ComplexityLanguage::Python => NodeKindTable::python(),
        ComplexityLanguage::Go => NodeKindTable::go(),
        ComplexityLanguage::Java => NodeKindTable::java(),
        ComplexityLanguage::C => NodeKindTable::c(),
        ComplexityLanguage::Cpp => NodeKindTable::cpp(),
        ComplexityLanguage::CSharp => NodeKindTable::csharp(),
        ComplexityLanguage::Php => NodeKindTable::php(),
    }
}

/// Integration entry point for a caller ([`crate::code_graph::CodeGraph::insert_file_and_chunks`])
/// that only has `(name, line)` pairs from a [`crate::parsers::ParsedFile`]
/// extraction pass, not a live AST handle: re-parses `source` under
/// `language`'s grammar once, then computes Tier A [`ComplexityMetrics`]
/// for every `(name, line)` pair in `symbols` that resolves to a
/// callable definition node. Symbols that do not resolve (e.g. a
/// non-callable symbol such as a type/class, or a stale name/line pair)
/// are simply absent from the returned map -- this function never
/// panics or errors on a resolution miss, matching this crate's
/// "never silent skip, but never panic either" extraction style
/// elsewhere (a missing map entry is the caller's signal to leave that
/// symbol's `metrics` field `None`).
pub fn metrics_for_symbols<'source>(
    language: ComplexityLanguage,
    source: impl Into<ComplexitySourceText<'source>>,
    symbols: impl AsRef<[ComplexitySymbolLocation]>,
) -> HashMap<ComplexitySymbolLocation, ComplexityMetrics> {
    let source = source.into();
    let source = source.as_str();
    let symbols = symbols.as_ref();
    let mut out = HashMap::new();
    let mut parser = Parser::new();
    if parser
        .set_language(&grammar_for_complexity(language))
        .is_err()
    {
        return out;
    }
    let Some(tree) = parser.parse(source, None) else {
        return out;
    };
    let table = node_kind_table(language);
    let src = source.as_bytes();
    let root = tree.root_node();
    for symbol in symbols {
        if let Some(def_node) =
            find_definition_node(root, symbol.name.retained(), symbol.line, src, &table)
        {
            out.insert(
                symbol.retained(),
                compute(def_node, symbol.name.retained(), src, &table),
            );
        }
    }
    out
}
