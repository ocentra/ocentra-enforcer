// RR-6.27
pub fn find_user(id: impl AsRef<str>) -> bool {
    !id.as_ref().is_empty()
}
