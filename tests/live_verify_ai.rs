#[cfg(feature = "live-tests")]
mod live {
    use std::process::Command;
    use std::fs;

    #[test]
    fn verify_ai_live_path_writes_snapshot_and_returns_results() {
        let token = std::env::var("COURTLISTENER_TOKEN")
            .expect("COURTLISTENER_TOKEN must be set");

        let output_dir = "out/live_test_verify";

        let status = Command::new("cargo")
            .args(["run", "--", "verify-ai", "fixtures/sample_brief.md", "fixtures/courtlistener_fixture.json", output_dir, "--live", "--token", &token,])
            .status()
            .expect("failed to run verify-ai");

        assert!(status.success());

        let dir = fs::read_dir(output_dir).expect("output dir missing");

        let mut found = false;
        for entry in dir {
            let entry = entry.unwrap();
            let path = entry.path();
            let content = fs::read_to_string(path).unwrap_or_default();

            if content.contains("snapshot_id") || content.contains("LookupRecord") {
                found = true;
            }
        }

        assert!(found, "no snapshot or lookup records found");
    }
}
