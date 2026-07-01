// RR-6.28
pub fn find_user(id: impl Into<String>) -> bool {
    !id.into().is_empty()
}
