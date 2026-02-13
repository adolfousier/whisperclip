use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::audio::Recorder;
use crate::config::Config;
use crate::db::Db;

const MIC_IDLE: &str = "\u{1F399}";
const MIC_REC: &str = "\u{23F9}";
const MIC_WAIT: &str = "\u{2026}";
const MIC_DONE: &str = "\u{2713}";

const CSS: &str = r#"
    window {
        background-color: transparent;
    }
    .mic-btn {
        min-width: 96px;
        min-height: 64px;
        border-radius: 14px;
        background-image: none;
        background-color: #dc2626;
        color: white;
        font-size: 28px;
        font-weight: 600;
        border: 2px solid rgba(255, 255, 255, 0.25);
        box-shadow: 0 4px 14px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(0, 0, 0, 0.15);
        outline: none;
        -gtk-icon-shadow: none;
        padding: 0;
    }
    .mic-btn:hover {
        background-image: none;
        background-color: #b91c1c;
        border: 2px solid rgba(255, 255, 255, 0.35);
    }
    .mic-btn:active {
        background-image: none;
        background-color: #991b1b;
    }
    .mic-btn.recording,
    .mic-btn.recording:hover {
        background-image: none;
        background-color: #16a34a;
        box-shadow: 0 4px 16px rgba(22, 163, 74, 0.5);
        animation: pulse 1s ease-in-out infinite;
    }
    .mic-btn.processing,
    .mic-btn.processing:hover {
        background-image: none;
        background-color: #d97706;
        box-shadow: 0 4px 12px rgba(217, 119, 6, 0.4);
    }
    .mic-btn.done,
    .mic-btn.done:hover {
        background-image: none;
        background-color: #16a34a;
        box-shadow: 0 4px 12px rgba(22, 163, 74, 0.4);
    }
    @keyframes pulse {
        0%   { opacity: 1.0; }
        50%  { opacity: 0.7; }
        100% { opacity: 1.0; }
    }
    .status-label {
        color: #e2e8f0;
        font-size: 9px;
        font-weight: 500;
        background-color: rgba(15, 23, 42, 0.75);
        border-radius: 6px;
        padding: 3px 8px;
    }
"#;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Idle,
    Recording,
    Processing,
}

pub fn build_ui(app: &gtk4::Application, config: Arc<Config>) {
    // Load CSS
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(CSS);
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("LinWhisper")
        .default_width(112)
        .default_height(96)
        .decorated(false)
        .resizable(false)
        .build();

    // Layout
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vbox.set_halign(gtk4::Align::Center);
    vbox.set_valign(gtk4::Align::Center);

    // The mic button
    let button = gtk4::Button::with_label(MIC_IDLE);
    button.add_css_class("mic-btn");
    button.set_size_request(96, 64);

    let status = gtk4::Label::new(None);
    status.add_css_class("status-label");
    status.set_visible(false);

    vbox.append(&button);
    vbox.append(&status);

    // WindowHandle wraps everything — makes the empty area around
    // the button draggable like a titlebar. Clicks on the Button
    // itself still go through to the button's click handler.
    let handle = gtk4::WindowHandle::new();
    handle.set_child(Some(&vbox));
    window.set_child(Some(&handle));

    // Open DB
    let db = Arc::new(Mutex::new(
        Db::open(&config.db_path).expect("Failed to open database"),
    ));

    // Shared state
    let state = Rc::new(RefCell::new(State::Idle));
    let recorder = Rc::new(RefCell::new(Recorder::new().expect("Failed to init audio")));
    let pending_paste = Rc::new(RefCell::new(false));

    // --- Left-click handler (on the Button) ---
    let btn = button.clone();
    let st = status.clone();
    let state_c = Rc::clone(&state);
    let rec_c = Rc::clone(&recorder);
    let config_c = Arc::clone(&config);
    let db_c = Arc::clone(&db);
    let pp = Rc::clone(&pending_paste);

    button.connect_clicked(move |_| {
        let current = *state_c.borrow();
        match current {
            State::Idle => {
                if let Err(e) = rec_c.borrow_mut().start() {
                    eprintln!("Record start error: {e}");
                    st.set_label(&format!("Err: {e}"));
                    st.set_visible(true);
                    return;
                }
                *state_c.borrow_mut() = State::Recording;
                btn.add_css_class("recording");
                btn.remove_css_class("done");
                btn.set_label(MIC_REC);
                st.set_label("Recording...");
                st.set_visible(true);
            }
            State::Recording => {
                *state_c.borrow_mut() = State::Processing;
                btn.remove_css_class("recording");
                btn.add_css_class("processing");
                btn.set_label(MIC_WAIT);
                st.set_label("Transcribing...");

                let wav = match rec_c.borrow_mut().stop() {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("Record stop error: {e}");
                        st.set_label(&format!("Err: {e}"));
                        *state_c.borrow_mut() = State::Idle;
                        btn.remove_css_class("processing");
                        btn.set_label(MIC_IDLE);
                        return;
                    }
                };

                let api_key = config_c.groq_api_key.clone();
                let model = config_c.groq_model.clone();
                let db_inner = Arc::clone(&db_c);

                let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    let result = rt.block_on(crate::api::transcribe(&api_key, &model, wav));
                    let _ = tx.send(result);
                });

                let btn2 = btn.clone();
                let st2 = st.clone();
                let state_c2 = Rc::clone(&state_c);
                let pp2 = Rc::clone(&pp);
                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    match rx.try_recv() {
                        Ok(Ok(text)) => {
                            if let Ok(db) = db_inner.lock() {
                                if let Err(e) = db.insert(&text) {
                                    eprintln!("DB insert error: {e}");
                                }
                            }
                            match crate::input::copy_to_clipboard(&text) {
                                Ok(_) => {
                                    eprintln!("Clipboard ready, waiting for focus change...");
                                    *pp2.borrow_mut() = true;
                                    btn2.remove_css_class("processing");
                                    btn2.add_css_class("done");
                                    btn2.set_label(MIC_DONE);
                                    st2.set_label("Click target \u{2192}");
                                }
                                Err(e) => {
                                    eprintln!("Clipboard error: {e}");
                                    btn2.remove_css_class("processing");
                                    btn2.set_label(MIC_IDLE);
                                    st2.set_label("Error!");
                                }
                            }
                            *state_c2.borrow_mut() = State::Idle;
                            glib::ControlFlow::Break
                        }
                        Ok(Err(e)) => {
                            eprintln!("Transcription error: {e}");
                            btn2.remove_css_class("processing");
                            btn2.set_label(MIC_IDLE);
                            st2.set_label("Error!");
                            let st3 = st2.clone();
                            glib::timeout_add_local_once(
                                std::time::Duration::from_secs(3),
                                move || st3.set_visible(false),
                            );
                            *state_c2.borrow_mut() = State::Idle;
                            glib::ControlFlow::Break
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                        Err(_) => {
                            *state_c2.borrow_mut() = State::Idle;
                            btn2.remove_css_class("processing");
                            btn2.set_label(MIC_IDLE);
                            glib::ControlFlow::Break
                        }
                    }
                });
            }
            State::Processing => {}
        }
    });

    // --- Focus loss: paste into target window ---
    let pp_focus = Rc::clone(&pending_paste);
    let st_focus = status.clone();
    let btn_focus = button.clone();
    window.connect_is_active_notify(move |_win| {
        if !_win.is_active() && *pp_focus.borrow() {
            *pp_focus.borrow_mut() = false;
            eprintln!("Focus lost — pasting...");
            let st_f = st_focus.clone();
            let btn_f = btn_focus.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                if let Err(e) = crate::input::simulate_paste() {
                    eprintln!("Paste error: {e}");
                }
                st_f.set_label("Pasted!");
                let st_f2 = st_f.clone();
                let btn_f2 = btn_f.clone();
                glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
                    st_f2.set_visible(false);
                    btn_f2.remove_css_class("done");
                    btn_f2.set_label(MIC_IDLE);
                });
            });
        }
    });

    // --- Right-click popover menu (on the button) ---
    // Use a PopoverMenu so it works inside WindowHandle
    // (WindowHandle steals raw right-clicks for the WM menu)
    let menu = gtk4::gio::Menu::new();
    menu.append(Some("History"), Some("app.show-history"));
    menu.append(Some("Quit"), Some("app.quit"));

    let popover = gtk4::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(&button);
    popover.set_has_arrow(true);

    // Right-click on button → show our popover, suppress WM menu
    let pop = popover.clone();
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    gesture.connect_pressed(move |g, _, _, _| {
        g.set_state(gtk4::EventSequenceState::Claimed);
        pop.popup();
    });
    button.add_controller(gesture);

    // Action: show history
    let history_action = gtk4::gio::SimpleAction::new("show-history", None);
    let db_hist = Arc::clone(&db);
    let win_ref = window.clone();
    history_action.connect_activate(move |_, _| {
        show_history_dialog(&win_ref, &db_hist);
    });
    app.add_action(&history_action);

    // Action: quit
    let quit_action = gtk4::gio::SimpleAction::new("quit", None);
    quit_action.connect_activate(move |_, _| {
        std::process::exit(0);
    });
    app.add_action(&quit_action);

    // --- Save position on close ---
    let db_close = Arc::clone(&db);
    window.connect_close_request(move |win| {
        save_window_position(win, &db_close);
        glib::Propagation::Proceed
    });

    // --- Position: saved or bottom-right, always-on-top ---
    let db_pos = Arc::clone(&db);
    window.connect_realize(move |win| {
        if let Some(surface) = win.surface() {
            if let Some(toplevel) = surface.downcast_ref::<gdk::Toplevel>() {
                toplevel.set_decorated(false);
            }
        }
        let w = win.clone();
        let db_p = Arc::clone(&db_pos);
        glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
            position_window(&w, &db_p);
        });
    });

    window.present();
}

/// Query current window position via xdotool and save to DB.
fn save_window_position(win: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let title = win.title().map(|t| t.to_string()).unwrap_or_default();
    if let Ok(output) = std::process::Command::new("xdotool")
        .args(["search", "--name", &title, "getwindowgeometry"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if let Some(pos) = line.strip_prefix("  Position: ") {
                if let Some((xs, ys)) = pos.split_once(',') {
                    let x = xs.trim();
                    let y = ys.split_whitespace().next().unwrap_or("0");
                    if let Ok(db) = db.lock() {
                        let _ = db.set_setting("window_x", x);
                        let _ = db.set_setting("window_y", y);
                    }
                }
            }
        }
    }
}

fn position_window(_window: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let saved = db.lock().ok().and_then(|db| {
        let x = db.get_setting("window_x").ok()??.parse::<i32>().ok()?;
        let y = db.get_setting("window_y").ok()??.parse::<i32>().ok()?;
        Some((x, y))
    });

    let (x, y) = match saved {
        Some(pos) => pos,
        None => {
            if let Some(display) = gdk::Display::default() {
                let monitors = display.monitors();
                if let Some(monitor) =
                    monitors.item(0).and_then(|m| m.downcast::<gdk::Monitor>().ok())
                {
                    let geom = monitor.geometry();
                    (
                        geom.x() + geom.width() - 100,
                        geom.y() + geom.height() - 140,
                    )
                } else {
                    (100, 100)
                }
            } else {
                (100, 100)
            }
        }
    };

    let title = "LinWhisper";
    let _ = std::process::Command::new("xdotool")
        .args([
            "search", "--name", title,
            "windowmove", &x.to_string(), &y.to_string(),
        ])
        .status();

    let _ = std::process::Command::new("xdotool")
        .args(["search", "--name", title, "windowactivate", "--sync"])
        .status();
    set_always_on_top(true);
}

fn set_always_on_top(on: bool) {
    let title = "LinWhisper";
    let action = if on { "add,above" } else { "remove,above" };
    let _ = std::process::Command::new("wmctrl")
        .args(["-r", title, "-b", action])
        .status();
}

fn show_history_dialog(_window: &gtk4::ApplicationWindow, db: &Arc<Mutex<Db>>) {
    let dialog = gtk4::Window::builder()
        .title("LinWhisper History")
        .default_width(400)
        .default_height(300)
        .build();

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);

    let header = gtk4::Label::new(Some("Recent Transcriptions"));
    header.add_css_class("heading");
    vbox.append(&header);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);

    let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    if let Ok(db) = db.lock() {
        if let Ok(entries) = db.recent(20) {
            if entries.is_empty() {
                let empty = gtk4::Label::new(Some("No transcriptions yet."));
                list_box.append(&empty);
            } else {
                for entry in entries {
                    let row = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
                    let time = gtk4::Label::new(Some(&entry.created_at));
                    time.set_halign(gtk4::Align::Start);
                    time.set_opacity(0.6);

                    let text = gtk4::Label::new(Some(&entry.text));
                    text.set_halign(gtk4::Align::Start);
                    text.set_wrap(true);
                    text.set_selectable(true);

                    row.append(&time);
                    row.append(&text);

                    let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                    list_box.append(&row);
                    list_box.append(&sep);
                }
            }
        }
    }

    scroll.set_child(Some(&list_box));
    vbox.append(&scroll);

    dialog.set_child(Some(&vbox));
    dialog.present();
}
