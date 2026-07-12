//! The h02 record boundary: the single place the `REQUIRED_TEST_CATEGORIES`
//! JSON record (wire shape documented on [`super`]) is deserialized and the
//! one module allowed to hold its raw wire fields. Everything downstream of
//! [`parse_record`] consumes typed accessors only — the raw `Vec<String>`
//! test-id arrays never leave this module as owned wire state, mirroring the
//! parse-at-boundary doctrine every other record-consuming validator in this
//! workspace follows.
//!
//! A malformed or non-JSON record is not this rule family's concern: an
//! invalid document yields `None` from [`parse_record`] and the validators
//! stay silent (the same contract h01/h03 document for unparseable source).
//!
//! BOUNDARY-INVARIANT: no `RequiredTestCategoriesRecord` value exists
//! without passing [`parse_record`]'s full serde decode below — every
//! inbound raw document is validated here, once; a malformed document is
//! rejected as `None` and never escapes this module as a value; the raw
//! wire fields stay private to this module and only typed accessors travel
//! inward to the h02 check functions.
//!
//! boundaryOwnerNote: h02 (`required-test-categories-gate`) owns this parse
//! boundary; it exists so the h02 rule module gains exactly ONE sanctioned
//! raw-record entry point instead of widening any crate-wide raw-string
//! ownership globs.
//
// PROPERTY-TEST: tests/required_test_categories_parity.rs
// (req_testcat_seven_property_over_all_category_subsets) drives the parse +
// gate pipeline across every one of the 128 category-presence subsets and
// across malformed/non-JSON inputs.

/// The seven required test categories from §4/§8.3, in spec order, each as
/// `(wire key, human label)`.
const REQUIRED_CATEGORIES: [(&str, &str); 7] = [
    ("negative", "negative"),
    ("replay", "replay"),
    ("concurrency", "concurrency"),
    ("rollback", "rollback/compensation"),
    ("economic_exhaustion", "economic-exhaustion"),
    ("time_based", "time-based"),
    ("signing", "signing/verification"),
];

/// One unit's category-tagged test ids, split by required category. Every
/// field is a count-bearing array (not a bare presence flag) so a future
/// consumer can also surface which specific test ids satisfy a category.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub(super) struct CategoryTests {
    // DEFAULT-JUSTIFICATION: an absent category array means "zero tests in
    // this category" — exactly the coverage gap this gate exists to detect.
    #[serde(default)]
    negative: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    replay: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    concurrency: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    rollback: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    economic_exhaustion: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    time_based: Vec<String>,
    // DEFAULT-JUSTIFICATION: absent category array == zero tests recorded.
    #[serde(default)]
    signing: Vec<String>,
}

impl CategoryTests {
    /// True when the named wire category carries zero test ids. An
    /// unrecognized key reads as empty (fails closed toward "missing").
    fn category_is_empty(&self, category_key: &str) -> bool {
        match category_key {
            "negative" => self.negative.is_empty(),
            "replay" => self.replay.is_empty(),
            "concurrency" => self.concurrency.is_empty(),
            "rollback" => self.rollback.is_empty(),
            "economic_exhaustion" => self.economic_exhaustion.is_empty(),
            "time_based" => self.time_based.is_empty(),
            "signing" => self.signing.is_empty(),
            _ => true,
        }
    }

    /// Every required category with a zero-length test list, labelled for
    /// display in spec order.
    fn missing_category_labels(&self) -> Vec<&'static str> {
        REQUIRED_CATEGORIES
            .iter()
            .filter(|(key, _)| self.category_is_empty(key))
            .map(|(_, label)| *label)
            .collect()
    }

    /// True when at least one category carries at least one test id — the
    /// minimal "resolves to at least one category-tagged test" bar the
    /// `REQ-TESTCAT-MAP.1` gate checks, distinct from the seven-category
    /// completeness `REQ-TESTCAT-SEVEN.1` checks.
    fn has_any_test(&self) -> bool {
        REQUIRED_CATEGORIES
            .iter()
            .any(|(key, _)| !self.category_is_empty(key))
    }
}

/// One `units` entry: a unit name paired with its category-tagged tests.
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct UnitEntry {
    unit: String,
    // DEFAULT-JUSTIFICATION: a unit entry with no `tests` object at all is a
    // legal wire state meaning "no categories recorded" — the gate reports
    // all seven as missing rather than rejecting the record.
    #[serde(default)]
    tests: Option<CategoryTests>,
}

impl UnitEntry {
    /// The unit's wire name.
    pub(super) fn name(&self) -> &str {
        &self.unit
    }

    /// Every required category label this unit is missing (all seven when
    /// the entry carries no `tests` object at all).
    pub(super) fn missing_category_labels(&self) -> Vec<&'static str> {
        match self.tests.as_ref() {
            Some(tests) => tests.missing_category_labels(),
            None => REQUIRED_CATEGORIES.iter().map(|(_, label)| *label).collect(),
        }
    }
}

/// The whole `REQUIRED_TEST_CATEGORIES` record: the h01-shaped
/// money-critical manifest snapshot (`moneyCriticalUnits`) and the per-unit
/// category-test mapping (`units`).
#[derive(Debug, Clone, serde::Deserialize)]
pub(super) struct RequiredTestCategoriesRecord {
    // DEFAULT-JUSTIFICATION: an absent manifest snapshot means "no
    // classified units" — the map gate then has nothing to assert over,
    // which is the correct silent outcome for an empty manifest.
    #[serde(rename = "moneyCriticalUnits", default)]
    money_critical_units: Vec<String>,
    // DEFAULT-JUSTIFICATION: an absent `units` list means "nothing mapped";
    // combined with a non-empty manifest snapshot that is precisely the
    // unresolved-unit state the map gate flags.
    #[serde(default)]
    units: Vec<UnitEntry>,
}

impl RequiredTestCategoriesRecord {
    /// Every mapped unit entry, in wire order.
    pub(super) fn units(&self) -> &[UnitEntry] {
        &self.units
    }

    /// The h01-shaped manifest snapshot of classified unit names, in wire
    /// order.
    pub(super) fn money_critical_units(&self) -> &[String] {
        &self.money_critical_units
    }

    /// True when `unit_name` resolves to a `units` entry carrying at least
    /// one category-tagged test id.
    pub(super) fn unit_resolves(&self, unit_name: &str) -> bool {
        self.units
            .iter()
            .find(|entry| entry.unit == unit_name)
            .and_then(|entry| entry.tests.as_ref())
            .is_some_and(CategoryTests::has_any_test)
    }
}

/// Parse `source` as a `REQUIRED_TEST_CATEGORIES` record. Unparseable,
/// invalid, or non-JSON source is not this validator family's concern
/// (mirrors h01's/h03's "unparseable source stays silent" contract) —
/// returns `None` rather than a `Finding`.
pub(super) fn parse_record(source: &str) -> Option<RequiredTestCategoriesRecord> {
    serde_json::from_str(source).ok()
}
