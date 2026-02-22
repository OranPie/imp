pub const ANNO_SAFE: &str = "safe";

pub fn is_core_target(target: &str) -> bool {
    target.starts_with("core::")
}

pub fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
