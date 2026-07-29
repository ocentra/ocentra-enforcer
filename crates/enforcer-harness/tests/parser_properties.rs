use std::path::Path;

use proptest::{prelude::any, proptest};

proptest! {
    #[test]
    fn every_recorded_parser_is_total_for_arbitrary_text(raw in any::<String>()) {
        let outcomes = (
            enforcer_harness::adapters::cyberskills::recorded::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::concurrency_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::coverage_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::fuzz_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::observability_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::static_analysis_report::parse_recorded(&raw).is_ok(),
        );
        let repeated = (
            enforcer_harness::adapters::cyberskills::recorded::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::concurrency_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::coverage_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::fuzz_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::observability_report::parse_recorded(&raw).is_ok(),
            enforcer_harness::security_pipeline::adapters::static_analysis_report::parse_recorded(&raw).is_ok(),
        );
        assert_eq!(outcomes, repeated);

        let severity = enforcer_harness::adapters::cyberskills::seam::AdapterOutcome::normalize_severity(&raw);
        assert!(matches!(
            severity,
            enforcer_domain::severity::Severity::Info
                | enforcer_domain::severity::Severity::Warning
                | enforcer_domain::severity::Severity::Error
        ));
    }

    #[test]
    fn manifest_and_diagnostic_parsers_are_total_for_arbitrary_text(raw in any::<String>()) {
        let local = enforcer_harness::ci_parity::parse_local_manifest(&raw);
        let local_repeated = enforcer_harness::ci_parity::parse_local_manifest(&raw);
        assert_eq!(local, local_repeated);
        let ci = enforcer_harness::ci_parity::parse_ci_manifest(&raw);
        let ci_repeated = enforcer_harness::ci_parity::parse_ci_manifest(&raw);
        assert_eq!(ci, ci_repeated);
        let diagnostics = enforcer_harness::parsers::parse_diagnostics("property-run", "property-tool", &raw, "");
        let diagnostics_repeated = enforcer_harness::parsers::parse_diagnostics("property-run", "property-tool", &raw, "");
        assert_eq!(diagnostics, diagnostics_repeated);
    }

    #[test]
    fn normalize_rel_is_total_for_arbitrary_path_components(component in "[^/\\\\]{0,64}") {
        let root = Path::new("property-root");
        let target = root.join(component);
        let normalized = enforcer_harness::legacy::normalize_rel(root, &target);
        assert_eq!(normalized, normalized.replace('\\', "/"));
    }
}
