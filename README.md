# Exile Companion

A native Windows/Linux Path of Exile companion written in Rust. The project is intentionally informational: it reads the client log, stores local session events, and displays statistics without controlling the game.

## Current features

- Native desktop dashboard
- Configurable `Client.txt` monitoring
- Automatic PoE 1 process detection and Windows/Steam/Proton log discovery
- Proton-safe detection of truncated Path of Exile process names
- Optional automatic monitoring when the game starts
- Automatic log connection even when the companion starts before or after the game
- Compact always-on-top in-game HUD for borderless-windowed gameplay that avoids covering the game
- Automatic overlay entry when PoE starts and dashboard restoration when it closes
- Full pages open in the normal app instead of expanding over the game
- In-game Assistant, Character, and Events tabs with one-click OCR capture and character review
- Active-character selector, live PoE/log indicators, session counters, and an explicit Exit control
- Local smart alerts for trades, deaths, level-ups, OCR results, character changes, and Ollama completion
- Structured trade cards with buyer, item, price, stash location, copy-reply, complete, and dismiss controls
- Local trade completion history and optional operating-system notifications
- Current-area and session timers calculated from local `Client.txt` events
- Captured-defense freshness, missing-data guidance, and local equipment comparison summaries
- OCR word-confidence scoring with review gates for uncertain or similar character names
- Persistent HUD opacity, size, lock state, and last screen position stored in SQLite
- Responsive navigation, cards, and spacing for narrow in-game layouts
- Borderless transparent in-game window with a custom drag region
- Native transparency with invisible resize zones on every HUD edge and corner
- Translucent glass text surfaces prevent game and companion labels from visually colliding
- Explicit interactive hit surface prevents compositor click-through in the in-game HUD
- Area, level-up, death, trade-whisper, and chat event parsing
- Immediate replay of the most recent 256 KiB of log history on connection
- Local SQLite event history
- Local Path of Building export-code and XML import with character stats, skills, and equipment
- Imported build snapshots remain available alongside live `Client.txt` session information
- Fully local character capture without PoE credentials or Path of Building
- Persistent multi-character roster with automatic OCR name matching and profile switching
- OCR recognition for visible name, level, class/ascendancy, league, and character-sheet totals
- User-triggered equipped-item clipboard parsing with recognized bonus totals
- One-click, user-triggered screen capture with local character-sheet OCR
- Optional screenshot-folder watcher and editable OCR text import
- Official passive-tree URL validation and allocation inspection
- SQLite character snapshot history with load and comparison controls
- Optional local Ollama assistant grounded in captured character data and the last 30 parsed log events
- One-click character review and optional automatic analysis after screenshot OCR
- Ollama thinking disabled, warm model retention, and explicit stale-knowledge safeguards
- Captured gem/link-group roster with level, quality, and tags included in local Ollama context
- Transparent local build assessment for captured Life/ES, resistance gaps, gem groups, and missing data
- Map-mod OCR with an editable local danger-phrase list
- SQLite map-run journal with duration, deaths, investment notes, and loot notes
- Local crafting worksheet based on copied item text, with explicit limits where affix data is unavailable
- Character progression checklist and passive-tree allocation ID inspection
- Local loot-filter editor, structural checks, and `.filter` export
- Full-screen, center-panel, and top-center OCR crop presets for more reliable capture
- Resolution-specific OCR calibration presets with grayscale, contrast, and local text upscaling
- Per-character map-risk profiles and editable Atlas/boss progression milestones
- Versioned, user-supplied local PoE JSON data packs for modifier rules, passives, gems, maps, bosses, and pantheons
- Bundled open core reference pack with optional user-supplied versioned overrides
- Defensive coverage dashboard for captured armour/evasion, suppression, block, recovery, ailments, and resistances
- Transparent captured defensive-readiness score with a visible 100-point breakdown and explicit non-simulation limits
- Local candidate-item comparison against the captured equipped slot
- Optional user-triggered poe.ninja market snapshots, cached locally by league with searchable currency/unique/gem/map estimates
- Guided five-source Capture Center with persisted timestamps and OCR confidence per character
- Custom percentage-based OCR crop calibration with a local image preview
- Optional local passive-tree JSON loading to resolve allocated node IDs into names
- Map-run averages, deathless rate, complete local history, and CSV export
- Complete JSON backup/merge restore for characters, snapshots, map runs, planners, filters, and crop settings
- First-run diagnostics for Client.txt, SQLite, Tesseract, screenshot folders, and optional Ollama
- Guided first-run local setup, manual public-GitHub update check, local panic log viewer, and version/database information
- Session counters and recent-event feed
- Explicit policy boundary: no game-input, memory-reading, packet, or unattended automation modules

## Run

Install the stable Rust toolchain, then:

```bash
cargo run -p poe-app
```

For the local assistant, start Ollama and install the default model:

```bash
ollama pull qwen3.5:2b
```

On Linux, eframe may require the normal X11/Wayland development packages supplied by your distribution. The database is saved under the platform data directory when available, otherwise beside the executable.

The app initially guesses common log paths. Use **Select Client.txt** to choose the correct file. For Steam/Proton this is typically inside the relevant Steam compatibility prefix.

Exile Companion never asks users to sign in. It does not use OAuth, API keys, `POESESSID`, Path of Exile credentials, or an Exile Companion account. Character data comes from user-triggered screenshots/clipboard actions and local files. The optional Ollama connection is restricted to localhost.

To load character information without an API key, open **Character** and either use the fully in-app capture workflow or, optionally, paste a Path of Building export code/select a saved PoB XML file. Web build links are not downloaded. Imports are parsed locally and do not disable `Client.txt` monitoring.

Path of Building is optional. For an entirely in-app workflow, open **Character** and:

1. Capture a screen where the character name/level header is visible. OCR selects the matching saved character or creates a new profile automatically; identity fields remain editable if OCR needs correction.
2. Hover each equipped item in PoE, press `Ctrl+C`, select its slot, and use **Read clipboard and capture**.
3. Copy each active/support gem and assign the same group label to gems linked together.
4. Open the in-game character sheet and click **Capture screen and read**, select an existing screenshot, or paste/edit OCR text manually. Choose the crop preset that best contains the visible panel.
5. Paste an official `pathofexile.com` passive-tree URL to inspect its encoded class and allocated node IDs locally.
6. Save local snapshots to compare the captured character later.

Once any character data is captured, select **Analyze with Ollama** for a local review. Enable **Automatically ask Ollama after a successful screenshot read** if you want the review to run immediately after OCR. The app sends the captured snapshot only to the locally configured Ollama endpoint; no PoE API key is needed.

Use the **Active character** selector to move between saved profiles or **New character** to create one manually. Equipment, sheet values, passive links, Ollama reviews, and snapshots are kept separate. If a screenshot does not visibly contain a recognizable character name, its sheet values are applied to the currently selected profile and the app says so in the OCR status.

## In-game HUD

Select **In-game HUD** or enable automatic HUD opening in Settings. The HUD stays compact and has four local tabs:

- **Assistant** — local Ollama shortcuts, latest answer, and an exact summary of the supplied local context.
- **Character** — active profile, OCR capture/review, captured defenses, freshness, equipment count, and the latest copied-item comparison.
- **Events** — session and current-area timers, counters, structured trade cards, and recent `Client.txt` activity.
- **HUD** — opacity, position/size locking, extra-compact mode, local data-source summary, and taskbar hiding.

Press `F10` while the companion has keyboard focus to toggle the HUD, or `Escape` to leave it. The HUD remembers its last position when exited. It never reads game memory or sends input to Path of Exile. Values labelled “captured” come from the most recent screenshot or clipboard action and are not live combat telemetry.

Screenshot recognition uses Tesseract locally and never uploads images. Windows and AppImage releases include the OCR engine and English recognition model, so release users do not need to install or configure Tesseract. Development builds prefer a bundled runtime when present and otherwise use the `tesseract` executable on `PATH`. One-click captures are written to a temporary image and deleted immediately after OCR. The optional folder watcher ignores existing images and processes only new screenshots after they finish saving. If OCR is unavailable, the editable OCR text box remains usable for manually supplied text.

## Local competitive toolkit

Open **Tools** for the map-mod checker, crafting/upgrade worksheet, map-run journal, progression checklist, defensive coverage, and loot-filter editor. The map checker can capture the top-center of the game screen with the same local OCR pipeline used by character capture. Risk phrases are stored per character; craft notes, filters, progression, completed map runs, and trade history stay local.

Settings can load an optional versioned local data-pack JSON or export a starter template. Packs are ordinary readable files, require no network access, and can supply modifier patterns, passive labels, gem tags, map/boss/pantheon lists, and crafting notes. Update checks are manual and read only the public GitHub release endpoint.

The bundled core pack is used automatically when no override is selected. The defensive-readiness score is deliberately inspectable and based only on captured values; it is not effective hit pool, DPS, or a combat simulator. Candidate-item comparisons likewise show their simple defensive weights and do not score offence or special mechanics.

The optional **Public market snapshot** in Tools downloads poe.ninja's public PoE 1 economy overview only after the user clicks refresh. It requests no account data, API key, OAuth token, or session cookie. Results are labelled as third-party estimates with their league/source/time, cached locally, included in backups, and never treated as guaranteed sale prices. Rare-item pricing is intentionally not guessed from incomplete local text.

The build assessment and upgrade comparison deliberately use only values the app actually captured. They do not reproduce Path of Building's combat simulation, infer hidden passive effects, download current affix/price data, or claim exact DPS from incomplete screenshots. This keeps the no-login workflow private and makes uncertain data visible instead of fabricating precision.

The progression review also checks captured gems for movement, guard, aura/reservation, and curse/mark coverage. These checks report only what has been captured; an absent result means “not observed,” not necessarily that the live character lacks it.

## Backup and release packages

Use **Settings → Backup and restore** to export a readable local JSON backup or merge one into the current installation. SQLite itself is compiled into the application, and the schema/database file is created automatically in the current user's private application-data directory. Existing working-directory databases are migrated on first launch. Settings also contains setup diagnostics, the exact database path, and any locally written panic log.

Tagged releases and manually dispatched GitHub Actions build a Linux AppImage and a Windows NSIS installer with `cargo-packager`. Both packages carry an offline English Tesseract runtime/data set, and the workflow runs the complete test suite before packaging.

Windows Authenticode signing is supported when repository secrets `WINDOWS_CERTIFICATE_BASE64` (base64 PFX contents) and `WINDOWS_CERTIFICATE_PASSWORD` are configured. Without them, the same workflow explicitly produces an unsigned installer. No signing material is stored in the repository.

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

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.
