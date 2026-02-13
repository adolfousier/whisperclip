# LinWhisper

Floating voice-to-text tool for Linux. Click to record, click to transcribe and paste - powered by Groq's Whisper API.

![LinWhisper button states](src/screenshots/ui-buttons.png)

## Privacy First

LinWhisper runs locally - no account, no telemetry, no background processes. Your microphone is **never accessed** until you explicitly click the record button. Audio is captured in-memory and sent to the Groq API for transcription, then immediately discarded. Raw audio is never written to disk and is not stored by Groq ([Groq's privacy policy](https://groq.com/privacy-policy/)). Only the transcribed text is stored locally in SQLite on your machine.

## Features

- Always-on-top floating microphone button (draggable, position persists)
- One-click voice recording with visual feedback (red idle, green recording)
- Transcription via Groq API (whisper-large-v3-turbo)
- Auto-pastes transcribed text into focused input field
- SQLite history with right-click access
- No background mic access - recording only on explicit click
- Audio stays in-memory, never saved to disk

## Dependencies

### System packages

**Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libgraphene-1.0-dev libvulkan-dev libasound2-dev xclip xdotool wmctrl
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 graphene vulkan-icd-loader alsa-lib xclip xdotool wmctrl
```

### Runtime requirements

- **X11 session required** - LinWhisper relies on `xdotool`, `xclip`, and `wmctrl` for window positioning, clipboard, and paste simulation. These are X11-only tools and **do not work on native Wayland**. If your distro runs Wayland by default (GNOME 41+, Fedora, etc.), you can either:
  - Log in to an X11/Xorg session from your display manager
  - Run under XWayland (may partially work, but not guaranteed)
- Working microphone

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/adolfousier/linwhisper.git
   cd linwhisper
   ```

2. Create a `.env` file with your Groq API key:
   ```
   GROQ_API_KEY=your_key_here
   GROQ_STT_MODEL=whisper-large-v3-turbo
   ```

3. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

## Usage

| Action | What happens |
|---|---|
| **Left-click** | Start recording (button turns green with pulse, shows stop icon) |
| **Left-click again** | Stop recording, transcribe, auto-paste into focused input |
| **Right-click** | Popover menu with History and Quit |
| **Drag** | Move the button anywhere on screen - position saved across sessions |

> **Note:** Auto-paste uses `xclip` and `xdotool` to simulate Ctrl+V. If text doesn't paste automatically, it will still be copied to your clipboard - just paste manually with Ctrl+V.

## Stack

| Component | Crate/Tool |
|-----------|-----------|
| GUI | gtk4-rs (GTK 4) |
| Audio | cpal + hound |
| API | reqwest (multipart) |
| Database | rusqlite (bundled SQLite) |
| Paste | xclip + xdotool |
| Config | dotenvy |

## License

MIT - see [LICENSE](LICENSE)
