// RR-3.16
pub fn cast(raw: u32) -> f32 {
    unsafe { std::mem::transmute(raw) }
}
