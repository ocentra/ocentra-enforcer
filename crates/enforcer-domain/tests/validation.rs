use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::{DartFilenameStem, DartWidgetName, McpReportLabelText};

#[test]
fn dart_widget_name_rejects_invalid_input() -> Result<(), DecodeError> {
    for invalid in ["", "widget", "Widget-Card", "Widget Card"] {
        assert!(
            DartWidgetName::try_new(invalid.to_owned()).is_err(),
            "invalid Dart widget name should be rejected: {invalid:?}"
        );
    }

    let valid = DartWidgetName::try_new("OrderCard".to_owned())?;
    assert_eq!(valid.as_str(), "OrderCard");
    Ok(())
}

#[test]
fn dart_filename_stem_rejects_invalid_input() -> Result<(), DecodeError> {
    for invalid in ["", "OrderCard", "order-card", "order card"] {
        assert!(
            DartFilenameStem::try_new(invalid.to_owned()).is_err(),
            "invalid Dart filename stem should be rejected: {invalid:?}"
        );
    }

    let valid = DartFilenameStem::try_new("order_card".to_owned())?;
    assert_eq!(valid.as_str(), "order_card");
    Ok(())
}

#[test]
fn mcp_report_label_text_rejects_invalid_blank_and_control_input() {
    for invalid in ["", "   ", "bad\nlabel", "bad\0label"] {
        assert!(McpReportLabelText::try_new(invalid.to_owned()).is_err());
    }
}

#[test]
fn mcp_report_label_text_preserves_epoch_fallback_edge_value() -> Result<(), DecodeError> {
    let label = McpReportLabelText::try_new("1970-01-01T00:00:00.000Z".to_owned())?;
    assert_eq!(label.into_inner(), "1970-01-01T00:00:00.000Z");
    Ok(())
}
