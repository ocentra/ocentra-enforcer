// RR-8.20
pub fn channel() {
    let _ = tokio::sync::mpsc::unbounded_channel::<String>();
}
