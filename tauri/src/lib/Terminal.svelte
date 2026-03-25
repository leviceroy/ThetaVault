<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import "@xterm/xterm/css/xterm.css";

  let container: HTMLDivElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let unlistenPty: (() => void) | null = null;
  let resizeObserver: ResizeObserver;
  let splashVisible = true;

  onMount(async () => {
    // ── 1. Build and open xterm ───────────────────────────────────────────────
    term = new Terminal({
      fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", Menlo, monospace',
      fontSize: 14,
      lineHeight: 1.2,
      convertEol: false,
      theme: {
        background:          "#0d1117",
        foreground:          "#e6edf3",
        cursor:              "#58a6ff",
        selectionBackground: "#264f78",
        black:         "#0d1117",
        red:           "#ff7b72",
        green:         "#3fb950",
        yellow:        "#d29922",
        blue:          "#58a6ff",
        magenta:       "#bc8cff",
        cyan:          "#39c5cf",
        white:         "#b1bac4",
        brightBlack:   "#6e7681",
        brightRed:     "#ffa198",
        brightGreen:   "#56d364",
        brightYellow:  "#e3b341",
        brightBlue:    "#79c0ff",
        brightMagenta: "#d2a8ff",
        brightCyan:    "#56d4dd",
        brightWhite:   "#f0f6fc",
      },
      cursorBlink:       true,
      allowTransparency: false,
      scrollback:        1000,
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);

    // Let the browser paint the container before measuring dimensions
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    fitAddon.fit();

    // ── 2. Forward keystrokes to PTY ──────────────────────────────────────────
    term.onData((data) => {
      invoke("pty_write", { data }).catch(console.error);
    });

    // ── 3. Register PTY data listener BEFORE calling pty_start ───────────────
    unlistenPty = await listen<number[]>("pty-data", (event) => {
      const payload = event.payload;
      if (Array.isArray(payload)) {
        term.write(new Uint8Array(payload));
      } else if (typeof payload === "string") {
        term.write(payload);
      }
      // Drop the splash overlay on first real frame — hides startup garbage
      if (splashVisible) splashVisible = false;
    });

    // ── 4. Tell the backend we're ready — releases the reader thread gate ─────
    const { rows, cols } = term;
    await invoke("pty_resize", { rows, cols });
    await invoke("pty_start");

    // ── 5. Resize observer ────────────────────────────────────────────────────
    // Skip the first fire (mount-time) to avoid sending ^L before the TUI is ready.
    let firstResize = true;
    resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      const { rows, cols } = term;
      if (firstResize) {
        firstResize = false;
        invoke("pty_resize", { rows, cols }).catch(console.error);
      } else {
        // Send Ctrl+L after resize so Ratatui redraws at the new size.
        invoke("pty_resize", { rows, cols })
          .then(() => invoke("pty_write", { data: "\x0c" }))
          .catch(console.error);
      }
    });
    resizeObserver.observe(container);
  });

  onDestroy(() => {
    unlistenPty?.();
    resizeObserver?.disconnect();
    term?.dispose();
  });
</script>

<div class="terminal-container" bind:this={container}></div>
{#if splashVisible}
  <div class="splash">
    <div class="splash-content">
      <pre class="splash-art">
 ████████╗██╗  ██╗███████╗████████╗ █████╗
    ██╔══╝██║  ██║██╔════╝╚══██╔══╝██╔══██╗
    ██║   ███████║█████╗     ██║   ███████║
    ██║   ██╔══██║██╔══╝     ██║   ██╔══██║
    ██║   ██║  ██║███████╗   ██║   ██║  ██║
    ╚═╝   ╚═╝  ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝

██╗   ██╗ █████╗ ██╗   ██╗██╗  ████████╗
██║   ██║██╔══██╗██║   ██║██║  ╚══██╔══╝
██║   ██║███████║██║   ██║██║     ██║
╚██╗ ██╔╝██╔══██║██║   ██║██║     ██║
 ╚████╔╝ ██║  ██║╚██████╔╝███████╗██║
  ╚═══╝  ╚═╝  ╚═╝ ╚═════╝ ╚══════╝╚═╝  </pre>
      <div class="splash-sub">Options Trading Journal</div>
    </div>
  </div>
{/if}

<style>
  .terminal-container {
    width: 100%;
    height: 100%;
    background: #0d1117;
    overflow: hidden;
  }

  .terminal-container :global(.xterm) {
    width: 100%;
    height: 100%;
    padding: 4px;
  }

  .terminal-container :global(.xterm-viewport) {
    overflow-y: hidden !important;
  }

  .splash {
    position: fixed;
    inset: 0;
    background: #0d1117;
    z-index: 9999;
    pointer-events: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .splash-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
  }

  .splash-art {
    margin: 0;
    padding: 0;
    font-family: "JetBrains Mono", "Fira Code", "Cascadia Code", Menlo, monospace;
    font-size: 13px;
    line-height: 1.4;
    color: #22c55e;
    text-shadow: 0 0 20px rgba(34, 197, 94, 0.4);
    white-space: pre;
  }

  .splash-sub {
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 12px;
    letter-spacing: 4px;
    color: #4b5563;
    text-transform: uppercase;
  }
</style>
