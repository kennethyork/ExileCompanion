# Exile Companion

A native Windows/Linux Path of Exile companion written in Rust. The project is intentionally informational: it reads the client log, stores local session events, and displays statistics without controlling the game.

## Current features

- Native desktop dashboard
- Configurable `Client.txt` monitoring
- Automatic PoE 1 process detection and Windows/Steam/Proton log discovery
- Proton-safe detection of truncated Path of Exile process names
- Optional automatic monitoring when the game starts
- Automatic log connection even when the companion starts before or after the game
- Compact always-on-top assistant overlay for borderless-windowed gameplay
- Automatic overlay entry when PoE starts and dashboard restoration when it closes
- Full resizable always-on-top in-game interface with optional compact mode
- Responsive navigation, cards, and spacing for narrow in-game layouts
- Borderless transparent in-game window with a custom drag region
- Native transparency enabled at window creation and custom borderless resize grip
- Translucent glass text surfaces prevent game and companion labels from visually colliding
- Explicit interactive hit surface prevents compositor click-through in full in-game mode
- Persistent full-app move handle and large immediate-press resize control
- Area, level-up, death, trade-whisper, and chat event parsing
- Immediate replay of the most recent 256 KiB of log history on connection
- Local SQLite event history
- Optional local Ollama assistant grounded in the last 30 parsed log events
- Ollama thinking disabled, warm model retention, and explicit stale-knowledge safeguards
- Session counters and recent-event feed
- Explicit policy boundary: no game-input, memory-reading, packet, or unattended automation modules

## Run

Install the stable Rust toolchain, then:

```bash
cargo run -p poe-app
```

For the local assistant, start Ollama and install the default model:

```bash
ollama pull qwen3:1.7b
```

On Linux, eframe may require the normal X11/Wayland development packages supplied by your distribution. The database is saved under the platform data directory when available, otherwise beside the executable.

The app initially guesses common log paths. Use **Select Client.txt** to choose the correct file. For Steam/Proton this is typically inside the relevant Steam compatibility prefix.

## Test

```bash
cargo test --workspace
```

## Safety model

This application does not send input to Path of Exile. New features must fit one of these categories:

- local calculation or planning;
- reading user-selected files such as `Client.txt`;
- user-triggered screenshot analysis;
- documented official Path of Exile APIs;
- display-only overlays and notifications.

Before distributing new integrations, verify them against the current Grinding Gear Games Terms of Use and developer policies.
