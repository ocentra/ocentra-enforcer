//! Data-driven CP00 negative-fixture mutations.
//!
//! BOUNDARY-INVARIANT: mutations affect only an in-memory JSON clone.
//! NEGATIVE-TEST: every table entry targets one closed schema invariant.

use serde_json::{json, Value};

use super::PROTECTED_CATALOG_ID;

struct MutationCase(
    &'static str,
    fn(&mut Value, usize, usize, usize) -> Result<(), Box<dyn std::error::Error>>,
);

pub(crate) fn mutate(
    mut root: Value,
    case_name: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    let records = root["records"]
        .as_array()
        .ok_or("records must be an array")?;
    let reviewed = position(records, "exploiting-mass-assignment-in-rest-apis")?;
    let unavailable = position(records, PROTECTED_CATALOG_ID)?;
    let partial = position(records, "analyzing-kubernetes-audit-logs")?;
    let direct_applied = DIRECT
        .iter()
        .find(|case| case.0 == case_name)
        .map(|case| (case.1)(&mut root, reviewed, unavailable, partial))
        .transpose()?
        .is_some();
    if direct_applied {
        return Ok(root);
    }
    let projection_applied = PROJECTION
        .iter()
        .find(|case| case.0 == case_name)
        .map(|case| (case.1)(&mut root, reviewed, unavailable, partial))
        .transpose()?
        .is_some();
    if projection_applied {
        return Ok(root);
    }
    if case_name == "correction-fork" || case_name == "correction-duplicate-successor" {
        append_pair(&mut root, partial)?;
        return Ok(root);
    }
    let correction = CORRECTIONS
        .iter()
        .find(|spec| spec.name == case_name)
        .ok_or_else(|| format!("unknown fixture case: {case_name}"))?;
    append_correction(
        &mut root,
        partial_or_reviewed(case_name, reviewed, partial),
        correction,
    )
    .map(|()| root)
}

fn position(records: &[Value], catalog_id: &str) -> Result<usize, Box<dyn std::error::Error>> {
    records
        .iter()
        .position(|record| record["catalogId"] == catalog_id)
        .ok_or_else(|| format!("fixture row missing: {catalog_id}").into())
}

fn partial_or_reviewed(case_name: &str, reviewed: usize, partial: usize) -> usize {
    usize::from(case_name == "correction-repeated-kind") * reviewed
        + usize::from(case_name != "correction-repeated-kind") * partial
}

const DIRECT: &[MutationCase] = &[
    MutationCase("duplicate-catalog-id", |root, _, _, _| {
        root["records"][1]["catalogId"] = root["records"][0]["catalogId"].clone();
        Ok(())
    }),
    MutationCase("duplicate-source-path", |root, _, _, _| {
        root["records"][1]["sourcePath"] = root["records"][0]["sourcePath"].clone();
        Ok(())
    }),
    MutationCase("empty-reviewed-components", |root, reviewed, _, _| {
        root["records"][reviewed]["components"] = json!([]);
        Ok(())
    }),
    MutationCase("reviewed-missing-components", |root, reviewed, _, _| {
        root["records"][reviewed]
            .as_object_mut()
            .ok_or("reviewed row must be an object")?
            .remove("components");
        Ok(())
    }),
    MutationCase("invalid-source-availability", |root, _, _, _| {
        root["records"][0]["sourceAvailability"] = json!("missing");
        Ok(())
    }),
    MutationCase("invalid-decomposition-state", |root, _, _, _| {
        root["records"][0]["decompositionState"] = json!("blocked");
        Ok(())
    }),
    MutationCase("invalid-component-kind", |root, reviewed, _, _| {
        root["records"][reviewed]["components"][0]["kind"] = json!("guess");
        Ok(())
    }),
    MutationCase("invalid-component-status", |root, reviewed, _, _| {
        root["records"][reviewed]["components"][0]["status"] = json!("done");
        Ok(())
    }),
    MutationCase("malformed-source-sha", |root, _, _, _| {
        root["records"][0]["sourceSha256"] = json!("ABC");
        Ok(())
    }),
    MutationCase("unavailable-has-source-sha", |root, _, unavailable, _| {
        root["records"][unavailable]["sourceSha256"] = json!("00");
        Ok(())
    }),
    MutationCase("unavailable-has-components", |root, _, unavailable, _| {
        root["records"][unavailable]["components"] = json!([{"componentId":"x"}]);
        Ok(())
    }),
    MutationCase("mechanical-missing-predicate", |root, reviewed, _, _| {
        root["records"][reviewed]["components"][0]
            .as_object_mut()
            .ok_or("reviewed component must be an object")?
            .remove("predicate");
        Ok(())
    }),
    MutationCase("mechanical-missing-not-proved", |root, reviewed, _, _| {
        root["records"][reviewed]["components"][0]
            .as_object_mut()
            .ok_or("reviewed component must be an object")?
            .remove("notProved");
        Ok(())
    }),
    MutationCase("stale-totals", |root, _, _, _| {
        root["totals"] = json!({"nativeMapped": 99});
        Ok(())
    }),
    MutationCase("protected-blob-drift", |root, _, unavailable, _| {
        root["records"][unavailable]["unavailableSource"]["trackedBlob"] = json!("00");
        Ok(())
    }),
    MutationCase("unsupported-schema", |root, _, _, _| {
        root["schemaVersion"] = json!(99);
        Ok(())
    }),
];

const PROJECTION: &[MutationCase] = &[
    MutationCase("unsupported-kind", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["presentKinds"] = json!([
            "native-predicate",
            "external-engine",
            "advisory",
            "manual",
            "unknown"
        ]);
        Ok(())
    }),
    MutationCase("unsupported-status", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["kindStatus"]["native-predicate"] =
            json!("implemented");
        Ok(())
    }),
    MutationCase("projection-count-mismatch", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["componentCount"] = json!(5);
        Ok(())
    }),
    MutationCase("projection-set-mismatch", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["missingKinds"] = json!(["native-predicate"]);
        Ok(())
    }),
    MutationCase("duplicate-projection-kind", |root, reviewed, _, _| {
        let present = root["records"][reviewed]["cp08Projection"]["presentKinds"]
            .as_array()
            .ok_or("projection presentKinds must be an array")?
            .clone();
        let first = present
            .first()
            .cloned()
            .ok_or("projection presentKinds must not be empty")?;
        let mut duplicate = present;
        duplicate.push(first);
        root["records"][reviewed]["cp08Projection"]["presentKinds"] = Value::Array(duplicate);
        Ok(())
    }),
    MutationCase("projection-status-mismatch", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["status"] = json!("partial");
        Ok(())
    }),
    MutationCase("projection-hash-mismatch", |root, reviewed, _, _| {
        root["records"][reviewed]["cp08Projection"]["provenanceChain"][0]["sourceSha256"] =
            json!("0".repeat(64));
        Ok(())
    }),
    MutationCase(
        "source-artifact-anchor-role-confusion",
        |root, reviewed, _, _| {
            root["records"][reviewed]["cp08Projection"]["provenanceChain"][0]["artifactAnchors"] =
                root["records"][reviewed]["sourceAnchors"].clone();
            Ok(())
        },
    ),
    MutationCase("source-hash-mismatch", |root, reviewed, _, _| {
        root["records"][reviewed]["source"]["sha256"] = json!("00");
        Ok(())
    }),
];

struct CorrectionSpec {
    name: &'static str,
    id: &'static str,
    tag: &'static str,
    present: &'static [&'static str],
    missing: &'static [&'static str],
    adds: &'static [&'static str],
}
const ALL_KINDS: &[&str] = &["native-predicate", "external-engine", "advisory", "manual"];
const FULL: &[&str] = ALL_KINDS;
const NONE: &[&str] = &[];
const CORRECTIONS: &[CorrectionSpec] = &[
    CorrectionSpec {
        name: "correction-orphan",
        id: "corr-orphan",
        tag: "3",
        present: FULL,
        missing: NONE,
        adds: &["external-engine"],
    },
    CorrectionSpec {
        name: "correction-cycle",
        id: "corr-cycle",
        tag: "5",
        present: FULL,
        missing: NONE,
        adds: &["external-engine"],
    },
    CorrectionSpec {
        name: "correction-repeated-kind",
        id: "corr-repeat",
        tag: "6",
        present: FULL,
        missing: NONE,
        adds: &["native-predicate"],
    },
    CorrectionSpec {
        name: "correction-conflicting-status",
        id: "corr-conflict",
        tag: "7",
        present: FULL,
        missing: NONE,
        adds: &["external-engine"],
    },
    CorrectionSpec {
        name: "correction-nonmonotonic",
        id: "corr-nonmonotonic",
        tag: "8",
        present: &["native-predicate", "advisory", "manual"],
        missing: &["external-engine"],
        adds: &["external-engine"],
    },
];

fn append_pair(root: &mut Value, partial: usize) -> Result<(), Box<dyn std::error::Error>> {
    let prior =
        root["records"][partial]["cp08Projection"]["provenanceChain"][0]["artifactSha256"].clone();
    append_entry(
        root,
        partial,
        "corr-001",
        prior.clone(),
        "1",
        &["native-predicate", "external-engine", "advisory", "manual"],
        &[],
        &["external-engine"],
    )?;
    append_entry(
        root,
        partial,
        "corr-002",
        prior,
        "2",
        FULL,
        NONE,
        &["external-engine"],
    )
}

fn append_correction(
    root: &mut Value,
    record_index: usize,
    spec: &CorrectionSpec,
) -> Result<(), Box<dyn std::error::Error>> {
    let prior = if spec.name == "correction-orphan" {
        json!("e".repeat(64))
    } else {
        root["records"][record_index]["cp08Projection"]["provenanceChain"][0]["artifactSha256"]
            .clone()
    };
    append_entry(
        root,
        record_index,
        spec.id,
        prior,
        spec.tag,
        spec.present,
        spec.missing,
        spec.adds,
    )
}

fn append_entry(
    root: &mut Value,
    record_index: usize,
    correction_id: &str,
    prior: Value,
    tag: &str,
    present: &[&str],
    missing: &[&str],
    adds: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = root["records"][record_index]["sourceSha256"]
        .as_str()
        .ok_or("source hash missing")?
        .to_owned();
    let entry = json!({"relation":"additive-correction", "batch":"correction-01", "artifactPath":"proof/cyberskills/cp08/corrections/correction-01/decomposition.json", "artifactSha256":tag.repeat(64), "sourceSha256":source_sha, "artifactAnchors":["# additive correction:L1"], "componentCount":present.len(), "presentKinds":present, "missingKinds":missing, "kindStatus":{"native-predicate":"proposed", "external-engine":"blocked", "advisory":"retained", "manual":"retained"}, "correctionId":correction_id, "priorArtifactSha256":prior, "addsKinds":adds});
    root["records"][record_index]["cp08Projection"]["provenanceChain"]
        .as_array_mut()
        .ok_or("provenance chain must be an array")?
        .push(entry);
    Ok(())
}
