use crate::config;

#[test]
fn model_urls_are_reachable() {
    // Verify that model download URLs return HTTP 200 (HEAD request)
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();

    for model in config::LOCAL_MODEL_PRESETS {
        let url = config::model_url(model.file_name);
        let resp = client.head(&url).send();
        match resp {
            Ok(r) => {
                assert!(
                    r.status().is_success() || r.status().is_redirection(),
                    "model {} URL returned {}: {}",
                    model.id,
                    r.status(),
                    url
                );
            }
            Err(e) => {
                // Network may not be available in CI; skip rather than fail
                eprintln!("SKIP: could not reach {} ({}): {}", model.id, url, e);
            }
        }
    }
}

#[test]
fn model_download_tiny_to_tempdir() {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();

    let model = config::find_local_model("local-tiny").unwrap();
    let url = config::model_url(model.file_name);

    let resp = match client.get(&url).send() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("SKIP: network unavailable: {e}");
            return;
        }
    };

    if !resp.status().is_success() {
        eprintln!("SKIP: download returned {}", resp.status());
        return;
    }

    let bytes = resp.bytes().unwrap();
    // Tiny model is ~75 MB; just verify we got a reasonable amount of data
    assert!(
        bytes.len() > 1_000_000,
        "downloaded model is suspiciously small: {} bytes",
        bytes.len()
    );

    // Write to temp dir and verify file
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(model.file_name);
    std::fs::write(&path, &bytes).unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::metadata(&path).unwrap().len(), bytes.len() as u64);
}
