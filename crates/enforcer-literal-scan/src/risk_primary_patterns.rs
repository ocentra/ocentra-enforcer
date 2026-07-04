use crate::risk_heuristics::{
    looks_like_event, looks_like_human_message, looks_like_id_or_key, looks_like_json_blob,
    looks_like_protocol, looks_like_route_or_url, looks_like_shell, looks_like_sql,
    looks_like_state_or_status,
};
use crate::RiskCategory;

pub(crate) fn pattern_category(text: &str) -> Option<RiskCategory> {
    if looks_like_shell(text) {
        return Some(RiskCategory::ShellFragment);
    }
    if looks_like_sql(text) {
        return Some(RiskCategory::SqlFragment);
    }
    if looks_like_json_blob(text) {
        return Some(RiskCategory::RawJsonBlob);
    }
    if looks_like_route_or_url(text) {
        return Some(RiskCategory::RouteOrUrl);
    }
    if looks_like_protocol(text) {
        return Some(RiskCategory::ProtocolHeaderOrMedia);
    }
    if looks_like_event(text) {
        return Some(RiskCategory::EventOrCommandName);
    }
    if looks_like_id_or_key(text) {
        return Some(RiskCategory::IdOrKeyName);
    }
    if looks_like_state_or_status(text) {
        return Some(RiskCategory::StateOrStatus);
    }
    if looks_like_human_message(text) {
        return Some(RiskCategory::HumanMessage);
    }
    None
}
