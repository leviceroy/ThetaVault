// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Condvar, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ── App state (Tauri managed) ─────────────────────────────────────────────────

struct AppState {
    writer:    Arc<Mutex<Box<dyn Write + Send>>>,
    master:    Arc<Mutex<Box<dyn MasterPty + Send>>>,
    /// Signals the reader thread to begin streaming (set true by pty_start command)
    ready:     Arc<(Mutex<bool>, Condvar)>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Called by the frontend once xterm.js has registered its "pty-data" listener.
/// Releases the reader thread gate so PTY output starts flowing.
#[tauri::command]
fn pty_start(state: tauri::State<Arc<AppState>>) {
    let (lock, cvar) = &*state.ready;
    let mut ready = lock.lock().unwrap();
    *ready = true;
    cvar.notify_all();
}

/// Write keystrokes from xterm.js into the PTY.
#[tauri::command]
fn pty_write(data: String, state: tauri::State<Arc<AppState>>) {
    if let Ok(mut w) = state.writer.lock() {
        let _ = w.write_all(data.as_bytes());
    }
}

/// Resize the PTY — SIGWINCH triggers Ratatui to redraw automatically.
#[tauri::command]
fn pty_resize(rows: u16, cols: u16, state: tauri::State<Arc<AppState>>) {
    if let Ok(m) = state.master.lock() {
        let _ = m.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
    }
}

// ── OSC filter — strips THETAVAULT sequences, extracts chart events ───────────

enum OscEvent {
    ChartShow(String),
    ChartHide,
    PerfShow(String),
    PerfHide,
    GuideShow(String),
    GuideHide,
}

/// Scan a buffer for `ESC ] 9999 ; THETAVAULT_CHART:<json> BEL` sequences.
/// Returns (clean_bytes_for_xterm, events, leftover_incomplete_osc).
/// If an OSC sequence starts but the BEL terminator hasn't arrived yet, the
/// incomplete sequence is returned as `leftover` to be prepended to the next read.
fn filter_osc_streaming(input: &[u8]) -> (Vec<u8>, Vec<OscEvent>, Vec<u8>) {
    let mut out    = Vec::with_capacity(input.len());
    let mut events = Vec::new();
    let mut i = 0;

    while i < input.len() {
        // Detect start of our custom OSC sequences (9999 = chart, 9998 = perf)
        if input[i] == 0x1b
            && i + 1 < input.len()
            && input[i + 1] == b']'
            && (input[i + 2..].starts_with(b"9999;THETAVAULT")
             || input[i + 2..].starts_with(b"9998;THETAVAULT")
             || input[i + 2..].starts_with(b"9997;THETAVAULT"))
        {
            // Look for the BEL terminator within the remaining input
            if let Some(bel_rel) = input[i..].iter().position(|&b| b == 0x07) {
                // Complete sequence found in this buffer
                let seq = std::str::from_utf8(&input[i + 2..i + bel_rel]).unwrap_or("");
                if let Some(payload) = seq.strip_prefix("9999;") {
                    if let Some(json) = payload.strip_prefix("THETAVAULT_CHART:") {
                        events.push(OscEvent::ChartShow(json.to_string()));
                    } else if payload == "THETAVAULT_CHART_CLOSE" {
                        events.push(OscEvent::ChartHide);
                    }
                } else if let Some(payload) = seq.strip_prefix("9998;") {
                    if let Some(json) = payload.strip_prefix("THETAVAULT_PERF:") {
                        events.push(OscEvent::PerfShow(json.to_string()));
                    } else if payload == "THETAVAULT_PERF_CLOSE" {
                        events.push(OscEvent::PerfHide);
                    }
                } else if let Some(payload) = seq.strip_prefix("9997;") {
                    if let Some(strategy) = payload.strip_prefix("THETAVAULT_GUIDE:") {
                        events.push(OscEvent::GuideShow(strategy.to_string()));
                    } else if payload == "THETAVAULT_GUIDE_CLOSE" {
                        events.push(OscEvent::GuideHide);
                    }
                }
                i += bel_rel + 1;
            } else {
                // Sequence is incomplete — carry the rest over to the next read
                let leftover = input[i..].to_vec();
                return (out, events, leftover);
            }
            continue;
        }
        out.push(input[i]);
        i += 1;
    }
    (out, events, Vec::new())
}

// ── Binary locator ────────────────────────────────────────────────────────────

fn find_binary() -> String {
    // TUI_BINARY_BASE is set at compile time by build.rs (e.g. .../theta-vault-rust/target)
    // Works on any machine — no hardcoded paths.
    let base = env!("TUI_BINARY_BASE");
    let (first, second) = if cfg!(debug_assertions) {
        (format!("{}/debug/theta-vault-rust", base), format!("{}/release/theta-vault-rust", base))
    } else {
        (format!("{}/release/theta-vault-rust", base), format!("{}/debug/theta-vault-rust", base))
    };
    let candidates = [
        // 1. Production: binary bundled next to the Tauri exe
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("theta-vault-rust")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().into_owned()),
        // 2. Dev: from theta-vault-rust/target/ (auto-built by build.rs)
        Some(first).filter(|p| std::path::Path::new(p).exists()),
        Some(second).filter(|p| std::path::Path::new(p).exists()),
    ];
    for path in candidates.into_iter().flatten() {
        return path;
    }
    "theta-vault-rust".to_string()
}

// ── PTY spawner ───────────────────────────────────────────────────────────────

fn spawn_pty(app: AppHandle) -> Arc<AppState> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system
        .openpty(PtySize { rows: 40, cols: 220, pixel_width: 0, pixel_height: 0 })
        .expect("Failed to open PTY");

    let binary = find_binary();
    let mut cmd = CommandBuilder::new(&binary);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("THETA_VAULT_TAURI", "1");
    // TUI_CWD is set at compile time by build.rs — points to theta-vault-rust dir
    // so trades.db is found in the same location as `cargo run` uses.
    cmd.cwd(env!("TUI_CWD"));

    pair.slave
        .spawn_command(cmd)
        .unwrap_or_else(|e| panic!("Failed to spawn '{}': {}", binary, e));

    let writer = pair.master.take_writer().expect("PTY writer");
    let mut reader = pair.master.try_clone_reader().expect("PTY reader");

    let ready = Arc::new((Mutex::new(false), Condvar::new()));

    let state = Arc::new(AppState {
        writer: Arc::new(Mutex::new(writer)),
        master: Arc::new(Mutex::new(pair.master)),
        ready:  ready.clone(),
    });

    // Reader thread: waits for pty_start(), then streams PTY output forever.
    std::thread::spawn(move || {
        // ── Wait until frontend is ready ──────────────────────────────────
        let (lock, cvar) = &*ready;
        let mut r = lock.lock().unwrap();
        while !*r {
            r = cvar.wait(r).unwrap();
        }
        drop(r);

        let mut buf     = vec![0u8; 16384];
        // Carryover for OSC sequences that span multiple reads
        let mut pending: Vec<u8> = Vec::new();
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => {
                    std::process::exit(0);
                }
                Ok(n) => {
                    // Prepend any incomplete sequence from the previous read
                    let input = if pending.is_empty() {
                        buf[..n].to_vec()
                    } else {
                        let mut combined = std::mem::take(&mut pending);
                        combined.extend_from_slice(&buf[..n]);
                        combined
                    };
                    let (clean, events, leftover) = filter_osc_streaming(&input);
                    pending = leftover;

                    if !clean.is_empty() {
                        let _ = app.emit("pty-data", clean);
                    }

                    for event in events {
                        match event {
                            OscEvent::ChartShow(json) => {
                                let _ = app.emit("chart-show", json);
                            }
                            OscEvent::ChartHide => {
                                let _ = app.emit("chart-hide", ());
                            }
                            OscEvent::PerfShow(json) => {
                                let _ = app.emit("perf-show", json);
                            }
                            OscEvent::PerfHide => {
                                let _ = app.emit("perf-hide", ());
                            }
                            OscEvent::GuideShow(strategy) => {
                                let _ = app.emit("guide-show", strategy);
                            }
                            OscEvent::GuideHide => {
                                let _ = app.emit("guide-hide", ());
                            }
                        }
                    }
                }
            }
        }
    });

    state
}

// ── Window control ────────────────────────────────────────────────────────────

/// Called by the frontend after the first PTY frame renders, so the window
/// appears already showing content (no black flash or garbage escape codes).
#[tauri::command]
fn show_main_window(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
    }
}

// ── Tauri entry ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let state = spawn_pty(app.handle().clone());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![pty_start, pty_write, pty_resize, show_main_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
