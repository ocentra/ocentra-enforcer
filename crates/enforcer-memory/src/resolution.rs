//! X06 core parity: type-aware call resolution.
//!
//! The baseline's biggest internal edge over this crate's original
//! `CodeAdjacency::resolve_callee` (a same-name best-effort match, see
//! `analysis/mod.rs`) is its LSP-hybrid, *type-aware* call resolution --
//! a method call `x.foo()` resolves through `x`'s declared/inferred
//! type to the correct `Type::foo` symbol, not just "some symbol named
//! `foo` somewhere in the graph." This module is a post-index pass that
//! builds a symbol registry over a whole [`CodeGraph`] snapshot and
//! resolves every recorded [`crate::code_graph::CallEdge`] into a
//! symbol-scoped, confidence-tagged [`ResolvedCall`] using the type/
//! container information [`crate::code_graph::InheritsEdge`]/
//! [`crate::code_graph::ImplementsEdge`]/[`crate::code_graph::DefinesEdge`]/
//! [`crate::code_graph::TypeRefEdge`] already carry.
//!
//! # Resolution ladder (never guesses silently)
//!
//! For each call, in order, the first strategy that produces a match
//! wins; the [`ResolutionConfidence`] on the result records *which*
//! strategy fired (or that none did):
//!
//! 1. **self/this** ([`ReceiverHint::SelfOrThis`]): the call resolves
//!    to a method DEFINEd by the enclosing symbol's own container type
//!    -- [`ResolutionConfidence::Resolved`] when exactly one candidate
//!    method exists on that type (walking [`InheritsEdge`] for an
//!    inherited method the type itself does not define), otherwise
//!    [`ResolutionConfidence::Ambiguous`] with every candidate kept.
//! 2. **typed local** ([`ReceiverHint::Identifier`]/[`NewExpression`]):
//!    the receiver's declared type is looked up via a
//!    [`TypeRefEdge`] recorded against the *enclosing* symbol (the
//!    closest this crate's syntactic extraction gets to a per-local
//!    type table without a real type checker -- see this module's
//!    "honest limitations" note below), then the same DEFINES/INHERITS
//!    walk as step 1 runs against that type.
//! 3. **import-aware**: an unqualified call whose callee name matches a
//!    symbol reachable through one of the call's file's own
//!    [`crate::code_graph::ImportEdge`]s (matched against the imported
//!    module's declared symbols by [`CodeGraph::symbol_nodes`] name) --
//!    [`ResolutionConfidence::Probable`] (cross-file import resolution
//!    is inherently best-effort without resolving the target build
//!    system's own module graph).
//! 4. **unique-name fallback**: exactly one callable symbol in the
//!    whole graph shares the callee's last name segment --
//!    [`ResolutionConfidence::Probable`] (this is
//!    [`crate::analysis::CodeAdjacency`]'s pre-existing behavior,
//!    preserved here as the least-informed rung of the ladder rather
//!    than replaced).
//! 5. **ambiguous/unresolved**: more than one candidate at any rung
//!    that this pass cannot disambiguate further is
//!    [`ResolutionConfidence::Ambiguous`] (every candidate kept, never
//!    silently narrowed to one); zero candidates anywhere is
//!    [`ResolutionConfidence::Unresolved`].
//!
//! # Honest limitations
//!
//! This is a *syntactic* registry+type pass, not a real type checker or
//! LSP: "the receiver's declared type" in step 2 is approximated by the
//! closest [`TypeRefEdge`] this crate's extractors happen to record
//! against the enclosing symbol (parameter/return-type annotations),
//! not full local-variable dataflow (e.g. `let x: Foo = ...; x.bar()`
//! resolves if `x`'s type annotation was captured as a signature
//! TYPE_REF-shaped construct by the extractor; a bare `let x = Foo::new()`
//! with no type annotation does NOT resolve through this rung -- it
//! falls through to the unique-name fallback instead of being guessed).
//! This is a deliberate, documented gap (matching this crate's
//! "unresolved, as-written" posture everywhere else) rather than a
//! silent wrong answer.

use crate::code_graph::{CallEdge, CodeGraph, CodeNode, SymbolNode};
use crate::parsers::ReceiverHint;
use std::collections::HashMap;

/// How confident [`resolve`] is in one [`ResolvedCall`]'s
/// [`ResolvedCall::candidates`]. Never silently narrowed: an
/// [`Ambiguous`](Self::Ambiguous) result keeps every candidate rather
/// than picking one arbitrarily.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionConfidence {
    /// Exactly one candidate, found via type-aware resolution (self/
    /// this, or a receiver's known declared type, including through
    /// INHERITS).
    Resolved,
    /// Exactly one candidate, but found via a lower-confidence rung
    /// (import-following or unique-name fallback) rather than a
    /// type-driven match.
    Probable,
    /// More than one candidate and this pass cannot disambiguate
    /// further -- every candidate is kept in
    /// [`ResolvedCall::candidates`].
    Ambiguous,
    /// No candidate found by any rung of the ladder.
    Unresolved,
}

/// One call edge's resolution result: the original edge (by index into
/// [`CodeGraph::calls`]-order, since [`CallEdge`] itself has no id) plus
/// the symbol(s) it resolved to and how confident that resolution is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCall {
    /// The enclosing symbol's stable id (`sym:<path>:<line>:<name>`),
    /// if [`CallEdge::from_symbol`] was populated by the extractor.
    /// `None` for a module-scope call with no enclosing symbol, or for
    /// a call recorded by an extractor that predates this field.
    pub from_symbol_id: Option<String>,
    /// Every symbol id this call could resolve to, per the ladder
    /// documented on this module. Empty iff `confidence` is
    /// [`ResolutionConfidence::Unresolved`]; more than one entry iff
    /// `confidence` is [`ResolutionConfidence::Ambiguous`].
    pub candidates: Vec<String>,
    pub confidence: ResolutionConfidence,
}

/// A symbol registry entry: everything [`resolve`] needs about one
/// callable/type symbol without re-scanning [`CodeGraph::nodes`] per
/// call.
#[derive(Debug, Clone)]
struct RegistryEntry {
    id: String,
    file_id: String,
    is_callable: bool,
}

/// The post-index registry+type resolution pass. Builds an internal
/// symbol registry once, then resolves every [`CodeGraph::calls`] edge
/// against it. Pure/read-only over `graph` -- callers that want the
/// results attached to the graph itself use
/// [`CodeGraph::resolved_calls`] (wired by
/// [`CodeGraph::index_repository`]), not this function directly, but
/// `resolve` is also exposed as a free function for callers (tests,
/// [`crate::analysis`]) that already have a [`CodeGraph`] and want a
/// fresh resolution pass without re-indexing.
pub fn resolve(graph: &CodeGraph) -> Vec<ResolvedCall> {
    let registry = build_registry(graph);
    graph
        .calls()
        .iter()
        .map(|call| resolve_one(graph, &registry, call))
        .collect()
}

/// Registry keyed by every angle a call needs to look a symbol up by:
/// name (possibly many candidates), plus id-based lookups for exact
/// containment/type answers.
struct Registry<'g> {
    /// name -> every symbol with that exact name, in graph order.
    by_name: HashMap<&'g str, Vec<RegistryEntry>>,
    /// symbol id -> the symbol's own id, name, containing file, kind.
    by_id: HashMap<&'g str, &'g SymbolNode>,
    /// container symbol id -> member symbol ids DEFINEs (Method, most
    /// commonly, but any member kind is recorded).
    members_of: HashMap<&'g str, Vec<&'g str>>,
    /// sub symbol id -> super type name(s) as written (INHERITS).
    supertypes_of: HashMap<&'g str, Vec<&'g str>>,
    /// symbol id -> type name(s) referenced in its own signature
    /// (TYPE_REF) -- the closest this crate's syntactic pass gets to
    /// "what type is this parameter/return/local", see module docs.
    type_refs_of: HashMap<&'g str, Vec<&'g str>>,
    /// file id -> (module_path as written) for every IMPORTS edge that
    /// file declares.
    imports_of: HashMap<&'g str, Vec<&'g str>>,
    /// file id -> rel_path, for import-target matching.
    file_rel_path: HashMap<&'g str, &'g str>,
}

fn build_registry(graph: &CodeGraph) -> Registry<'_> {
    let mut by_name: HashMap<&str, Vec<RegistryEntry>> = HashMap::new();
    let mut by_id: HashMap<&str, &SymbolNode> = HashMap::new();

    for node in graph.nodes() {
        if let Some(symbol) = symbol_of(node) {
            by_id.insert(symbol.id.as_str(), symbol);
            by_name
                .entry(symbol.name.as_str())
                .or_default()
                .push(RegistryEntry {
                    id: symbol.id.clone(),
                    file_id: symbol.file_id.clone(),
                    is_callable: is_callable(node),
                });
        }
    }

    let mut members_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.defines() {
        members_of
            .entry(edge.container_id.as_str())
            .or_default()
            .push(edge.member_id.as_str());
    }

    let mut supertypes_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.inherits() {
        supertypes_of
            .entry(edge.sub_id.as_str())
            .or_default()
            .push(edge.super_name.as_str());
    }

    let mut type_refs_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.type_refs() {
        type_refs_of
            .entry(edge.from_id.as_str())
            .or_default()
            .push(edge.type_name.as_str());
    }

    let mut imports_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in graph.imports() {
        imports_of
            .entry(edge.from_file_id.as_str())
            .or_default()
            .push(edge.module_path.as_str());
    }

    let mut file_rel_path: HashMap<&str, &str> = HashMap::new();
    for file in graph.file_nodes() {
        file_rel_path.insert(file.id.as_str(), file.rel_path.as_str());
    }

    Registry {
        by_name,
        by_id,
        members_of,
        supertypes_of,
        type_refs_of,
        imports_of,
        file_rel_path,
    }
}

fn resolve_one(graph: &CodeGraph, registry: &Registry<'_>, call: &CallEdge) -> ResolvedCall {
    let from_symbol_id = call
        .from_symbol
        .as_ref()
        .zip(call.from_symbol_line)
        .and_then(|(name, line)| sym_id_for(graph, &call.from_file_id, line, name));

    // Rung 1 + 2: self/this or a typed receiver -- both resolve through
    // "the enclosing symbol's container type's members" once the
    // container type name is known; only how that type name is found
    // differs.
    if let Some(hint) = call.receiver_hint {
        let container_type_name = match hint {
            ReceiverHint::SelfOrThis => enclosing_container_type(graph, from_symbol_id.as_deref()),
            ReceiverHint::Identifier | ReceiverHint::NewExpression => {
                typed_receiver_type_name(registry, from_symbol_id.as_deref())
            }
            ReceiverHint::Literal | ReceiverHint::Other => None,
        };
        if let Some(type_name) = container_type_name {
            let method_name = last_segment(&call.callee);
            if let Some(result) =
                resolve_method_on_type(registry, &type_name, method_name, from_symbol_id.clone())
            {
                return result;
            }
            // A known container type but no matching member (possibly
            // because the type itself is defined in a file this graph
            // hasn't indexed, e.g. a third-party dependency) falls
            // through to the lower rungs below rather than being
            // reported Unresolved outright -- the unique-name fallback
            // may still legitimately find it.
        }
    }

    // Rung 3: import-aware resolution for an unqualified call whose
    // name matches a symbol declared in one of this call's file's own
    // imported modules.
    if let Some(result) = resolve_via_imports(registry, call, from_symbol_id.clone()) {
        return result;
    }

    // Rung 4: unique-name fallback, matching
    // `analysis::CodeAdjacency::resolve_callee`'s pre-existing
    // exact-or-last-segment name match, now confidence-tagged.
    resolve_via_unique_name(registry, &call.callee, from_symbol_id)
}

/// The symbol node for a [`CodeNode`], if it is any symbol-shaped
/// variant (every variant except `File`/`TextOnly`/`Tombstone`).
fn symbol_of(node: &CodeNode) -> Option<&SymbolNode> {
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
        | CodeNode::Constant(s) => Some(s),
        CodeNode::File(_) | CodeNode::TextOnly(_) | CodeNode::Tombstone(_) => None,
    }
}

/// Whether a [`CodeNode`] is callable in the same sense
/// `code_graph::callable_symbol` uses (Function/Method/Test/Lambda) --
/// re-derived here (rather than exported from `code_graph`) since that
/// helper is private to its own module and this crate's convention is
/// "each module owns its own small predicates over `CodeNode`" (see
/// `analysis::test_node_ids`'s identical re-derivation).
fn is_callable(node: &CodeNode) -> bool {
    matches!(
        node,
        CodeNode::Function(_) | CodeNode::Method(_) | CodeNode::Test(_) | CodeNode::Lambda(_)
    )
}

fn last_segment(callee: &str) -> &str {
    callee
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(callee)
        .trim_end_matches(['(', ')'])
}

/// Rebuild a symbol's stable id from its name+line the same way
/// [`CodeGraph::insert_file_and_chunks`] builds it
/// (`sym:<rel_path>:<line>:<name>`), then confirm it actually exists in
/// the registry (a call recorded against a symbol this graph does not
/// itself contain -- should not happen in practice, but this pass never
/// fabricates an id it cannot verify).
fn sym_id_for(graph: &CodeGraph, file_id: &str, line: usize, name: &str) -> Option<String> {
    let rel_path = graph
        .file_nodes()
        .find(|f| f.id == file_id)
        .map(|f| f.rel_path.as_str())?;
    let candidate = format!("sym:{rel_path}:{line}:{name}");
    graph
        .symbol_nodes()
        .any(|s| s.id == candidate)
        .then_some(candidate)
}

/// The name of the type that DEFINEs `from_symbol_id` (i.e. the
/// enclosing method's own container), for `self`/`this` resolution.
fn enclosing_container_type(graph: &CodeGraph, from_symbol_id: Option<&str>) -> Option<String> {
    let from_symbol_id = from_symbol_id?;
    graph
        .defines()
        .iter()
        .find(|edge| edge.member_id == from_symbol_id)
        .and_then(|edge| graph.symbol_nodes().find(|s| s.id == edge.container_id))
        .map(|s| s.name.clone())
}

/// The declared type name of the enclosing symbol's own receiver, as
/// approximated by its nearest TYPE_REF -- see this module's "honest
/// limitations" doc for exactly what this does and does not capture.
fn typed_receiver_type_name(
    registry: &Registry<'_>,
    from_symbol_id: Option<&str>,
) -> Option<String> {
    let from_symbol_id = from_symbol_id?;
    registry
        .type_refs_of
        .get(from_symbol_id)
        .and_then(|types| types.first())
        .map(|t| t.to_string())
}

/// Resolve `method_name` as a member of the type named `type_name`,
/// walking [`Registry::supertypes_of`] for an inherited method when
/// `type_name`'s own [`Registry::members_of`] does not define it
/// directly. Returns `None` (not `Unresolved`) when `type_name` itself
/// cannot be found in the registry at all, so the caller can fall
/// through to a lower rung instead of reporting a hard miss for a type
/// this graph simply does not index (e.g. a stdlib/third-party type).
fn resolve_method_on_type(
    registry: &Registry<'_>,
    type_name: &str,
    method_name: &str,
    from_symbol_id: Option<String>,
) -> Option<ResolvedCall> {
    let type_candidates = registry.by_name.get(type_name)?;
    if type_candidates.is_empty() {
        return None;
    }

    let mut found: Vec<String> = Vec::new();
    let mut visited_types: std::collections::HashSet<String> = std::collections::HashSet::new();
    for type_entry in type_candidates {
        collect_method_on_type_and_supertypes(
            registry,
            &type_entry.id,
            method_name,
            &mut found,
            &mut visited_types,
        );
    }
    found.sort();
    found.dedup();

    if found.is_empty() {
        // The type is known but has no matching member anywhere in its
        // INHERITS chain this graph can see -- let the caller fall
        // through to a lower rung rather than reporting Unresolved
        // (the member might be defined in an unindexed base, e.g. a
        // stdlib trait).
        return None;
    }

    Some(match found.len() {
        1 => ResolvedCall {
            from_symbol_id,
            candidates: found,
            confidence: ResolutionConfidence::Resolved,
        },
        _ => ResolvedCall {
            from_symbol_id,
            candidates: found,
            confidence: ResolutionConfidence::Ambiguous,
        },
    })
}

fn collect_method_on_type_and_supertypes(
    registry: &Registry<'_>,
    type_id: &str,
    method_name: &str,
    found: &mut Vec<String>,
    visited_types: &mut std::collections::HashSet<String>,
) {
    if !visited_types.insert(type_id.to_string()) {
        return; // cycle guard (an INHERITS cycle should not happen, but
                // this pass never trusts input data to be well-formed).
    }
    if let Some(members) = registry.members_of.get(type_id) {
        for member_id in members {
            if let Some(symbol) = registry.by_id.get(*member_id) {
                if symbol.name == method_name {
                    found.push((*member_id).to_string());
                }
            }
        }
    }
    if let Some(supertype_names) = registry.supertypes_of.get(type_id) {
        for super_name in supertype_names {
            if let Some(super_candidates) = registry.by_name.get(*super_name) {
                for super_entry in super_candidates {
                    collect_method_on_type_and_supertypes(
                        registry,
                        &super_entry.id,
                        method_name,
                        found,
                        visited_types,
                    );
                }
            }
        }
    }
}

/// Rung 3: an unqualified call resolves through the call's own file's
/// IMPORTS edges to a symbol declared in the imported module, matched
/// by the callee's last name segment against every symbol declared in a
/// file whose `rel_path` plausibly matches the import's module path
/// (same best-effort suffix/stem match `analysis::resolve_module_path`
/// already uses for import edges, re-applied here at the symbol level).
fn resolve_via_imports(
    registry: &Registry<'_>,
    call: &CallEdge,
    from_symbol_id: Option<String>,
) -> Option<ResolvedCall> {
    let imports = registry.imports_of.get(call.from_file_id.as_str())?;
    if imports.is_empty() {
        return None;
    }
    let method_name = last_segment(&call.callee);
    let name_candidates = registry.by_name.get(method_name)?;

    let mut matched: Vec<String> = Vec::new();
    for candidate in name_candidates {
        if !candidate.is_callable {
            continue;
        }
        let Some(candidate_rel_path) = registry.file_rel_path.get(candidate.file_id.as_str())
        else {
            continue;
        };
        let candidate_file_is_this_call_own_file = candidate.file_id == call.from_file_id;
        if candidate_file_is_this_call_own_file {
            continue; // same-file matches are the unique-name rung's job.
        }
        if imports
            .iter()
            .any(|module_path| import_matches_file(module_path, candidate_rel_path))
        {
            matched.push(candidate.id.clone());
        }
    }
    matched.sort();
    matched.dedup();

    match matched.len() {
        0 => None,
        1 => Some(ResolvedCall {
            from_symbol_id,
            candidates: matched,
            confidence: ResolutionConfidence::Probable,
        }),
        _ => Some(ResolvedCall {
            from_symbol_id,
            candidates: matched,
            confidence: ResolutionConfidence::Ambiguous,
        }),
    }
}

/// Same best-effort suffix/stem match as
/// `analysis::resolve_module_path`, re-implemented here at the
/// registry level (that function is private to `analysis` and takes a
/// petgraph index, not a rel_path, so it is not directly reusable).
fn import_matches_file(module_path: &str, rel_path: &str) -> bool {
    let needle = module_path
        .trim_start_matches("./")
        .trim_start_matches("../");
    let last_segment = needle.rsplit(['/', ':', '.']).next().unwrap_or(needle);
    if last_segment.is_empty() {
        return false;
    }
    let stem = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let stem = stem.split('.').next().unwrap_or(stem);
    stem == last_segment || rel_path.ends_with(last_segment)
}

/// Rung 4: exact-name or last-segment-name match against every
/// callable symbol in the graph, same rule
/// `analysis::resolve_callee` already applies for the pre-existing
/// file-granular `Calls` edge -- reproduced here so a caller using this
/// module standalone (without going through `analysis::CodeAdjacency`)
/// still gets identical fallback behavior.
fn resolve_via_unique_name(
    registry: &Registry<'_>,
    callee: &str,
    from_symbol_id: Option<String>,
) -> ResolvedCall {
    let last = last_segment(callee);
    let mut matched: Vec<String> = Vec::new();
    for (name, candidates) in &registry.by_name {
        if *name == callee || *name == last {
            for candidate in candidates {
                if candidate.is_callable {
                    matched.push(candidate.id.clone());
                }
            }
        }
    }
    matched.sort();
    matched.dedup();

    match matched.len() {
        0 => ResolvedCall {
            from_symbol_id,
            candidates: matched,
            confidence: ResolutionConfidence::Unresolved,
        },
        1 => ResolvedCall {
            from_symbol_id,
            candidates: matched,
            confidence: ResolutionConfidence::Probable,
        },
        _ => ResolvedCall {
            from_symbol_id,
            candidates: matched,
            confidence: ResolutionConfidence::Ambiguous,
        },
    }
}
