fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(enforcer_memory::runtime_probe::write_runtime_probe_stdout()?)
}
