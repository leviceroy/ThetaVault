# ThetaVault 2.0 (Rust Edition)

A high-performance, terminal-based options trading journal inspired by `sandbox-quant` and the tastytrade mechanics.

## 🚀 Why Rust?
- **Zero Latency:** Instant startup and UI updates.
- **Portability:** Compiles to a single binary. No Node.js or Docker required.
- **Reliability:** Strict typing ensures your trade math is always correct.

## 🛠 Tech Stack
- **UI:** `ratatui` (TUI library)
- **Database:** `SQLite` (Single-file portable database)
- **Runtime:** `Tokio` (Async processing)

## 🏁 Getting Started

1. **Install Rust:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Run the App:**
   ```bash
   cargo run
   ```

3. **Build Release Binary:**
   ```bash
   cargo build --release
   # Binary located at target/release/theta-vault-rust
   ```

## ⌨️ Controls
- `Tab` / `T`: Switch Tabs
- `Q`: Quit
- `N`: New Trade (Coming Soon)
- `Enter`: View Trade Details (Coming Soon)
