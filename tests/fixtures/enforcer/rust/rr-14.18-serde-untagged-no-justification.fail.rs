// RR-14.18
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub enum UserEvent {
    Created { id: String },
}
