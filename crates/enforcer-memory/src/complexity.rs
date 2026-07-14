//! X06 core parity: per-function complexity signals.
//!
//! Mirrors the C baseline's two-tier complexity pipeline
//! (`codebase-memory-mcp/src/pipeline/pass_parallel.c::build_def_props`
//! for Tier A, `pass_complexity.c::cbm_pipeline_pass_complexity` for
//! Tier B — see `docs/plans/enforcer-selfhost-plan/refs/
//! x06-baseline-tool-schemas.md` §4.5 for the authoritative property
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
    pub decision_points: &'static [&'static str],
    /// Loop node kinds (`for`, `while`, `loop`, comprehension-with-`for`).
    pub loops: &'static [&'static str],
    /// Nesting-increasing node kinds for cognitive complexity (loops +
    /// conditionals + `catch`/`except` + nested closures/functions).
    pub nesting: &'static [&'static str],
    /// The function/method definition node kind(s) themselves (used to
    /// recognize a nested function boundary while walking, so a
    /// closure's own decision points are not double counted into the
    /// enclosing function AND the metric still detects nesting).
    pub function_defs: &'static [&'static str],
    /// Call-expression node kind(s).
    pub calls: &'static [&'static str],
    /// The field name (on a call-expression node) holding the callee.
    pub call_function_field: &'static str,
    /// Member/property/field access expression node kind(s), used for
    /// `max_access_depth` (chained `a.b.c.d`).
    pub member_access: &'static [&'static str],
    /// The field name (on a member-access node) holding the base being
    /// accessed (its "left"/"object" side) -- walking this field
    /// repeatedly measures chain depth.
    pub member_access_object_field: &'static str,
    /// Parameter-list node kind(s) for a function/method definition.
    pub parameters: &'static [&'static str],
    /// The field name on a function/method definition node holding its
    /// parameter list.
    pub parameters_field: &'static str,
    /// The field name on a function/method definition node holding its
    /// own name (used to detect self-calls for recursion).
    pub name_field: &'static str,
    /// The field name on a function/method definition node holding its
    /// body block.
    pub body_field: &'static str,
    /// If-statement/expression node kind(s) (used separately from
    /// `decision_points` to walk the `else`/`elif` chain for cognitive
    /// complexity's "each additional branch, not just the first, adds
    /// nesting-weighted cost" rule).
    pub if_statements: &'static [&'static str],
    /// The field name on an if-statement node holding its `else`
    /// branch (may itself be another if-statement for `else if`).
    pub if_alternative_field: &'static str,
}

impl NodeKindTable {
    pub fn rust() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    pub fn typescript_javascript() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    pub fn python() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    pub fn go() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    pub fn java() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    /// C: `switch`/`case_statement` (matching the workpack's "decisions
    /// incl. switch cases" instruction -- each `case`/`default` arm adds
    /// 1, not the `switch_statement` itself), `for`/`while`/`do` loops,
    /// `->`/`.`-chained [`field_expression`] access, and pointer/`new`-
    /// style allocation callees (`malloc`/`calloc`/`realloc`, matched
    /// via [`ALLOC_CALLEES`] plus C's own idioms below).
    pub fn c() -> Self {
        Self {
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
            if_alternative_field: "alternative",
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
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    /// C#: `switch_expression_arm`/`switch_section` count as decision
    /// points (one per arm/case, matching `switch_case`'s treatment
    /// elsewhere), `foreach_statement` joins `for`/`while`/`do` as a
    /// loop, and lambdas/local functions extend the nesting/function-
    /// boundary sets the same way closures do for every other language.
    pub fn csharp() -> Self {
        Self {
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
            if_alternative_field: "alternative",
        }
    }

    /// PHP: `switch` `case_statement`s, `foreach`/`for`/`while`/`do`
    /// loops, `->`/`?->`-chained [`member_access_expression`] access,
    /// and closures/arrow-functions extend the nesting/function-
    /// boundary sets.
    pub fn php() -> Self {
        Self {
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
            if_alternative_field: "alternative",
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
    pub complexity: u32,
    pub cognitive: u32,
    pub loop_count: u32,
    pub loop_depth: u32,
    pub param_count: u32,
    pub max_access_depth: u32,
    pub linear_scan_in_loop: u32,
    pub alloc_in_loop: u32,
    /// Tier A "self_recursive" seed in the C baseline: a *direct*
    /// self-call only. [`TransitiveMetrics::recursive`] is the
    /// separate, OR'd-in Tier B property -- see this module's doc
    /// comment and the baseline doc's "naming hazard" note. Do not
    /// collapse the two.
    pub self_recursive: bool,
    pub recursion_in_loop: bool,
    pub unguarded_recursion: bool,
}

/// Tier B metrics, populated by [`propagate_transitive_loop_depth`]
/// after the whole repository's call graph is known -- kept as a
/// separate type from [`ComplexityMetrics`] because it is computed in
/// a distinct pass (interprocedural, not per-function) and, per the
/// baseline's naming-hazard note, `recursive` here is semantically
/// different from `self_recursive` above (self-recursion OR a
/// call-graph cycle via mutual recursion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransitiveMetrics {
    pub transitive_loop_depth: u32,
    pub recursive: bool,
}

/// Compute Tier A metrics for one function/method definition node.
///
/// `function_name` is the symbol's own name (used to recognize direct
/// self-calls for `self_recursive`/`recursion_in_loop`/
/// `unguarded_recursion` -- a call whose callee text's last path
/// segment equals this name). `src` is the whole file's source bytes
/// (tree-sitter nodes are byte-range views into it).
pub fn compute(
    def_node: Node<'_>,
    function_name: &str,
    src: &[u8],
    table: &NodeKindTable,
) -> ComplexityMetrics {
    let body = def_node
        .child_by_field_name(table.body_field)
        .unwrap_or(def_node);

    let mut metrics = ComplexityMetrics {
        complexity: 1, // cyclomatic complexity baseline: one path through the function.
        param_count: param_count(def_node, table),
        ..ComplexityMetrics::default()
    };

    let mut ctx = WalkCtx {
        table,
        src,
        function_name,
        metrics: &mut metrics,
    };
    walk_body(body, &mut ctx, 0, 0, false);
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
pub fn find_definition_node<'tree>(
    root: Node<'tree>,
    name: &str,
    line: usize,
    src: &[u8],
    table: &NodeKindTable,
) -> Option<Node<'tree>> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if table.function_defs.contains(&node.kind()) && node.start_position().row + 1 == line {
            if let Some(name_node) = node.child_by_field_name(table.name_field) {
                // Exact-field text match (every wave-A/B language table
                // so far: `name_field` names a leaf identifier node
                // directly, e.g. Rust/TS/Python/Go/Java's own `name`
                // field). Kept as the primary, unchanged path so no
                // existing language's behavior shifts by even one
                // node.
                if name_node.utf8_text(src).ok() == Some(name) {
                    return Some(node);
                }
                // C/C++ fallback: `name_field` (`"declarator"`) names a
                // *nested* declarator subtree (`function_declarator`,
                // possibly wrapped in `pointer_declarator`s for a
                // pointer-returning function) whose own `.utf8_text()`
                // is the whole declarator (`"foo(int x)"`), not the
                // bare name -- so this walks that subtree's innermost
                // identifier instead of comparing the field's own text.
                if let Some(inner_name) = innermost_identifier_text(name_node, src) {
                    if inner_name == name {
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
fn innermost_identifier_text<'a>(node: Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" | "destructor_name"
        | "operator_name" => node.utf8_text(src).ok(),
        "function_declarator"
        | "pointer_declarator"
        | "reference_declarator"
        | "parenthesized_declarator" => node
            .child_by_field_name("declarator")
            .and_then(|inner| innermost_identifier_text(inner, src)),
        "qualified_identifier" => node
            .child_by_field_name("name")
            .and_then(|inner| innermost_identifier_text(inner, src)),
        _ => None,
    }
}

fn param_count(def_node: Node<'_>, table: &NodeKindTable) -> u32 {
    let direct = def_node.child_by_field_name(table.parameters_field);
    // C/C++ fallback: `function_definition`'s own fields are
    // `type`/`declarator`/`body` -- the parameter list lives one level
    // deeper, on the `declarator` (`function_declarator`)'s own
    // `parameters` field, not directly on `def_node`. Every other
    // language's table already resolves `parameters_field` directly on
    // `def_node` (this `direct` lookup succeeds and the fallback is
    // never reached for them).
    let params = match direct {
        Some(params) if table.parameters.contains(&params.kind()) => params,
        _ => {
            let Some(nested) = def_node
                .child_by_field_name(table.name_field)
                .and_then(|declarator| declarator.child_by_field_name(table.parameters_field))
            else {
                return 0;
            };
            nested
        }
    };
    if !table.parameters.contains(&params.kind()) {
        return 0;
    }
    let mut count = 0u32;
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
    src: &'src [u8],
    function_name: &'a str,
    metrics: &'a mut ComplexityMetrics,
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
    loop_depth: u32,
    cognitive_nesting: u32,
    in_nested_fn: bool,
) {
    let kind = node.kind();

    if in_nested_fn {
        // Still need to descend to find further nested boundaries, but
        // stop attributing metrics to the outer function.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            walk_body(child, ctx, loop_depth, cognitive_nesting, true);
        }
        return;
    }

    let is_loop = ctx.table.loops.contains(&kind);
    let is_decision = ctx.table.decision_points.contains(&kind);
    let is_nesting = ctx.table.nesting.contains(&kind);

    if is_decision {
        ctx.metrics.complexity += 1;
    }
    if is_nesting {
        ctx.metrics.cognitive += 1 + cognitive_nesting;
    }

    let child_loop_depth = if is_loop {
        ctx.metrics.loop_count += 1;
        let new_depth = loop_depth + 1;
        if new_depth > ctx.metrics.loop_depth {
            ctx.metrics.loop_depth = new_depth;
        }
        new_depth
    } else {
        loop_depth
    };
    let child_cognitive_nesting = if is_nesting {
        cognitive_nesting + 1
    } else {
        cognitive_nesting
    };

    if ctx.table.calls.contains(&kind) {
        inspect_call(node, ctx, loop_depth);
    }

    let entering_nested_fn = ctx.table.function_defs.contains(&kind);

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

fn inspect_call(call_node: Node<'_>, ctx: &mut WalkCtx<'_, '_>, loop_depth: u32) {
    let Some(function_node) = call_node.child_by_field_name(ctx.table.call_function_field) else {
        return;
    };
    let Ok(callee_text) = function_node.utf8_text(ctx.src) else {
        return;
    };
    let last_segment = last_path_segment(callee_text);

    if last_segment == ctx.function_name {
        ctx.metrics.self_recursive = true;
        if loop_depth > 0 {
            ctx.metrics.recursion_in_loop = true;
        }
        if !recursion_is_guarded(call_node, ctx.table) {
            ctx.metrics.unguarded_recursion = true;
        }
    }

    if loop_depth > 0 {
        if LINEAR_SCAN_CALLEES.contains(&last_segment) {
            ctx.metrics.linear_scan_in_loop += 1;
        }
        if ALLOC_CALLEES.contains(&last_segment) {
            ctx.metrics.alloc_in_loop += 1;
        }
    }
}

/// The last `.`/`::`-separated segment of a callee expression's text,
/// e.g. `self.items.iter().find` -> `find`, `helper` -> `helper`.
fn last_path_segment(callee_text: &str) -> &str {
    callee_text
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(callee_text)
        .trim_end_matches(['(', ')'])
}

/// Whether a self-recursive call is reachable only through a
/// conditionally-guarded path, i.e. some ancestor between the call and
/// the function body is an if-statement/conditional (a base case
/// exists). Walking ancestors rather than requiring a literal `return`
/// keeps this check simple and matches the baseline's "no
/// conditionally-guarded base case" wording -- an unconditional
/// self-call at the top level of the function body (no enclosing `if`)
/// is unguarded infinite recursion by construction.
fn recursion_is_guarded(call_node: Node<'_>, table: &NodeKindTable) -> bool {
    let mut current = call_node.parent();
    while let Some(node) = current {
        if table.if_statements.contains(&node.kind())
            || table.decision_points.contains(&node.kind())
        {
            return true;
        }
        if table.function_defs.contains(&node.kind()) {
            break; // reached the enclosing function boundary; stop.
        }
        current = node.parent();
    }
    false
}

/// Max chained member/property access depth (`a.b.c.d` -> 4) anywhere
/// in the subtree. Walks each member-access node's "object" side
/// repeatedly to measure one chain's depth, and takes the max across
/// every chain found in the subtree (a function with several unrelated
/// short chains and one long one is scored by the long one).
fn max_access_depth(node: Node<'_>, table: &NodeKindTable) -> u32 {
    let mut best = 0u32;
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        // `compute` reports metrics for one definition only.  `walk_body`
        // already treats nested closures/functions as a boundary, so access
        // depth must do the same: a chain inside a closure belongs to that
        // closure's symbol, not its enclosing function.
        if table.function_defs.contains(&current.kind()) && current.id() != node.id() {
            continue;
        }
        if table.member_access.contains(&current.kind()) {
            let depth = chain_depth(current, table);
            if depth > best {
                best = depth;
            }
            // Do not descend further into an already-measured chain's
            // own object side (chain_depth already walked it) -- but
            // do still walk sibling/non-object children (e.g. call
            // arguments) for other chains.
            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                let is_object_side = current
                    .child_by_field_name(table.member_access_object_field)
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

fn chain_depth(node: Node<'_>, table: &NodeKindTable) -> u32 {
    let mut depth = 1u32;
    let mut current = node;
    while table.member_access.contains(&current.kind()) {
        depth += 1;
        match current.child_by_field_name(table.member_access_object_field) {
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
#[derive(Debug, Clone)]
pub struct CallGraphNode {
    pub id: String,
    pub loop_depth: u32,
    pub self_recursive: bool,
    pub callees: Vec<String>,
}

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
pub fn propagate_transitive_loop_depth(
    nodes: &[CallGraphNode],
) -> HashMap<String, TransitiveMetrics> {
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
        by_id: &'a HashMap<&'a str, &'a CallGraphNode>,
        state: HashMap<String, State>,
        tld: HashMap<String, u32>,
        recursive: HashMap<String, bool>,
        active_path: Vec<String>,
        max_depth: u32,
    }

    fn dfs(id: &str, depth: u32, ctx: &mut DfsCtx<'_>) -> u32 {
        let Some(node) = ctx.by_id.get(id).copied() else {
            return 0;
        };
        match ctx.state.get(id).copied().unwrap_or(State::Unvisited) {
            State::Done => return ctx.tld.get(id).copied().unwrap_or(0),
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
                return 0;
            }
            State::Unvisited => {}
        }
        if depth > ctx.max_depth {
            return node.loop_depth;
        }
        ctx.state.insert(id.to_string(), State::InProgress);
        ctx.active_path.push(id.to_string());
        let mut best = 0u32;
        for callee in &node.callees {
            if callee == id {
                ctx.recursive.insert(id.to_string(), true);
                continue;
            }
            let callee_tld = dfs(callee, depth + 1, ctx);
            if callee_tld > best {
                best = callee_tld;
            }
        }
        // Transitive depth is a bounded risk estimate. Saturate instead of
        // panicking in debug or wrapping in release when a caller supplies
        // an extreme graph metric.
        let value = node.loop_depth.saturating_add(best);
        ctx.active_path.pop();
        ctx.tld.insert(id.to_string(), value);
        ctx.state.insert(id.to_string(), State::Done);
        value
    }

    let by_id: HashMap<&str, &CallGraphNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let state: HashMap<String, State> = nodes
        .iter()
        .map(|n| (n.id.clone(), State::Unvisited))
        .collect();
    let recursive: HashMap<String, bool> = nodes
        .iter()
        .map(|n| (n.id.clone(), n.self_recursive))
        .collect();
    let mut ctx = DfsCtx {
        by_id: &by_id,
        state,
        tld: HashMap::new(),
        recursive,
        active_path: Vec::new(),
        max_depth: MAX_PROPAGATION_DEPTH,
    };

    for node in nodes {
        if ctx.state.get(node.id.as_str()).copied() != Some(State::Done) {
            dfs(&node.id, 0, &mut ctx);
        }
    }
    let tld = ctx.tld;
    let recursive = ctx.recursive;

    nodes
        .iter()
        .map(|n| {
            let metrics = TransitiveMetrics {
                transitive_loop_depth: tld.get(n.id.as_str()).copied().unwrap_or(n.loop_depth),
                recursive: recursive.get(n.id.as_str()).copied().unwrap_or(false),
            };
            (n.id.clone(), metrics)
        })
        .collect()
}

/// Which language a [`NodeKindTable`] should be built for -- mirrors
/// [`crate::parsers::Language`]'s structural-extractor subset without
/// this module depending on that type directly (keeps `complexity.rs`
/// usable/testable standalone, per this lane's file claim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityLanguage {
    Rust,
    TypeScriptOrJavaScript,
    Python,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    Php,
}

impl ComplexityLanguage {
    fn table(self) -> NodeKindTable {
        match self {
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

    fn grammar(self) -> tree_sitter::Language {
        match self {
            ComplexityLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ComplexityLanguage::TypeScriptOrJavaScript => {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
            ComplexityLanguage::Python => tree_sitter_python::LANGUAGE.into(),
            ComplexityLanguage::Go => tree_sitter_go::LANGUAGE.into(),
            ComplexityLanguage::Java => tree_sitter_java::LANGUAGE.into(),
            ComplexityLanguage::C => tree_sitter_c::LANGUAGE.into(),
            ComplexityLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            ComplexityLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            ComplexityLanguage::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        }
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
pub fn metrics_for_symbols(
    language: ComplexityLanguage,
    source: &str,
    symbols: &[(String, usize)],
) -> HashMap<(String, usize), ComplexityMetrics> {
    let mut out = HashMap::new();
    let mut parser = Parser::new();
    if parser.set_language(&language.grammar()).is_err() {
        return out;
    }
    let Some(tree) = parser.parse(source, None) else {
        return out;
    };
    let table = language.table();
    let src = source.as_bytes();
    let root = tree.root_node();
    for (name, line) in symbols {
        if let Some(def_node) = find_definition_node(root, name, *line, src, &table) {
            out.insert((name.clone(), *line), compute(def_node, name, src, &table));
        }
    }
    out
}
