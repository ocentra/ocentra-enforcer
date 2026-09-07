use enforcer_domain::boundary::decode_error::DecodeError;
use enforcer_domain::boundary::validation::{DartFilenameStem, DartWidgetName, McpReportLabelText};

#[test]
fn dart_widget_name_rejects_invalid_input() -> Result<(), DecodeError> {
    for invalid in ["", "widget", "Widget-Card", "Widget Card"] {
        let error = DartWidgetName::try_new(invalid.to_owned())
            .err()
            .ok_or_else(|| DecodeError::new("test.dartWidgetName", "invalid value was accepted"))?;
        assert_eq!(error.path, "dartWidgetName");
        assert_eq!(error.reason, "must be a public Dart identifier");
    }

    let valid = DartWidgetName::try_new("OrderCard".to_owned())?;
    assert_eq!(valid.as_str(), "OrderCard");
    Ok(())
}

#[test]
fn dart_filename_stem_rejects_invalid_input() -> Result<(), DecodeError> {
    for invalid in ["", "OrderCard", "order-card", "order card"] {
        let error = DartFilenameStem::try_new(invalid.to_owned())
            .err()
            .ok_or_else(|| {
                DecodeError::new("test.dartFilenameStem", "invalid value was accepted")
            })?;
        assert_eq!(error.path, "dartFilenameStem");
        assert_eq!(error.reason, "must be snake_case");
    }

    let valid = DartFilenameStem::try_new("order_card".to_owned())?;
    assert_eq!(valid.as_str(), "order_card");
    Ok(())
}

#[test]
fn mcp_report_label_text_rejects_invalid_blank_and_control_input() -> Result<(), DecodeError> {
    for invalid in ["", "   ", "bad\nlabel", "bad\0label"] {
        let error = McpReportLabelText::try_new(invalid.to_owned())
            .err()
            .ok_or_else(|| DecodeError::new("test.mcpReportLabel", "invalid value was accepted"))?;
        assert_eq!(error.path, "mcpReportLabel");
        assert!(
            matches!(
                error.reason.as_str(),
                "label is blank" | "label contains a control character"
            ),
            "unexpected validation reason: {}",
            error.reason
        );
    }
    Ok(())
}

#[test]
fn mcp_report_label_text_preserves_epoch_fallback_edge_value() -> Result<(), DecodeError> {
    let label = McpReportLabelText::try_new("1970-01-01T00:00:00.000Z".to_owned())?;
    assert_eq!(label.into_inner(), "1970-01-01T00:00:00.000Z");
    Ok(())
}
