// X06.9 parity/benchmark harness fixture repo: a tiny, deterministic
// synthetic corpus the QA row runners index and query against. Kept
// separate from tests/fixtures/memory/code_graph/ (X06.2's own fixture
// repo) per this lane's file claims (tests/fixtures/memory/** new only).

pub fn parse_config_file(path: &str) -> Config {
    read_config(path)
}

fn read_config(path: &str) -> Config {
    Config { path: path.to_string() }
}

pub struct Config {
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_file_returns_the_requested_path() {
        let config = parse_config_file("app.toml");
        assert_eq!(config.path, "app.toml");
    }
}
