
#[test]
fn cli_binary_help_is_declared() {
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(manifest.contains("name = \"citeright\""));
}
