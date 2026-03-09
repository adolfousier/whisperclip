//! # WhisperCrabs
//!
//! Floating voice-to-text tool for Linux, macOS, and Windows.
//!
//! Click to record, click to transcribe, text copied to clipboard.
//! Supports fully local transcription via whisper.cpp or any
//! OpenAI-compatible API endpoint (Groq, Ollama, OpenRouter, LM Studio, etc.).
//!
//! ## Features
//!
//! - Floating always-on-top mic button (GTK4)
//! - One-click provider switching via right-click menu
//! - Local transcription via whisper.cpp (no internet required)
//! - API transcription via any OpenAI-compatible endpoint
//! - Global keyboard shortcuts via D-Bus
//! - AI Agent-Ready: full D-Bus control

#[macro_use]
mod log;
mod api;
mod audio;
mod config;
mod db;
mod input;
mod local_stt;
#[cfg(test)]
mod tests;
mod tts;
mod ui;

use gtk4::prelude::*;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let debug = args.iter().any(|a| a == "--debug");
    log::init(debug);

    let config = Arc::new(config::Config::load());

    let app = gtk4::Application::builder()
        .application_id("dev.whispercrabs.app")
        .build();

    let config_c = Arc::clone(&config);
    app.connect_activate(move |app| {
        ui::build_ui(app, Arc::clone(&config_c));
    });

    // Filter out --debug so GTK4 doesn't reject it as unknown option
    let gtk_args: Vec<String> = args.into_iter().filter(|a| a != "--debug").collect();
    let gtk_args_ref: Vec<&str> = gtk_args.iter().map(|s| s.as_str()).collect();
    app.run_with_args(&gtk_args_ref);
}
