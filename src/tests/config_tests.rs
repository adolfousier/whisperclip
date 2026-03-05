use crate::config;

#[test]
fn find_preset_returns_known_providers() {
    let groq = config::find_preset("groq");
    assert!(groq.is_some());
    let groq = groq.unwrap();
    assert_eq!(groq.id, "groq");
    assert!(groq.needs_key);
    assert!(!groq.base_url.is_empty());

    let ollama = config::find_preset("ollama");
    assert!(ollama.is_some());
    assert!(!ollama.unwrap().needs_key);
}

#[test]
fn find_preset_returns_none_for_unknown() {
    assert!(config::find_preset("nonexistent").is_none());
}

#[test]
fn all_api_presets_have_required_fields() {
    for preset in config::API_PRESETS {
        assert!(!preset.id.is_empty(), "preset id must not be empty");
        assert!(!preset.label.is_empty(), "preset label must not be empty");
        assert!(
            preset.base_url.starts_with("http"),
            "preset {} base_url must start with http",
            preset.id
        );
        assert!(
            !preset.default_model.is_empty(),
            "preset {} default_model must not be empty",
            preset.id
        );
    }
}

#[test]
fn find_local_model_returns_known_models() {
    let tiny = config::find_local_model("local-tiny");
    assert!(tiny.is_some());
    let tiny = tiny.unwrap();
    assert_eq!(tiny.file_name, "ggml-tiny.en.bin");

    let medium = config::find_local_model("local-medium");
    assert!(medium.is_some());
}

#[test]
fn find_local_model_returns_none_for_unknown() {
    assert!(config::find_local_model("local-nonexistent").is_none());
}

#[test]
fn all_local_model_presets_have_required_fields() {
    for model in config::LOCAL_MODEL_PRESETS {
        assert!(!model.id.is_empty());
        assert!(!model.label.is_empty());
        assert!(
            model.file_name.ends_with(".bin"),
            "model {} file_name should end with .bin",
            model.id
        );
        assert!(!model.size_label.is_empty());
    }
}

#[test]
fn default_local_model_is_valid() {
    assert!(config::find_local_model(config::DEFAULT_LOCAL_MODEL).is_some());
}

#[test]
fn model_url_produces_valid_huggingface_url() {
    let url = config::model_url("ggml-tiny.en.bin");
    assert!(url.starts_with("https://huggingface.co/"));
    assert!(url.ends_with("ggml-tiny.en.bin"));
}
