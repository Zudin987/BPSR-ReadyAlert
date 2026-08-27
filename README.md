# BPSR Ready Alert

**BPSR Ready Alert** is a lightweight Windows companion for **Blue Protocol: Star Resonance (BPSR)**. It keeps the original Ready Check / matchmaking sound alerts and adds an optional floating **Chat Overlay** with filters, custom tabs, keyword sounds, smoother scrolling, and per-user sender colors.

**Website:** https://zudin987.github.io/projects/readyalert/

**Current stable release:** v1.1.2  
**Development candidate:** v1.2.0-rc.1

## Highlights

- Ready Check and matchmaking / party-confirm sound alerts.
- Optional floating Chat Overlay using the same capture pipeline — no second Npcap engine.
- World, Guild/Team, All, and custom chat tabs.
- Channel selection, minimum-level filters, Show/Hide expressions, OR/AND matching, and advanced regex.
- Up to 3 prioritized message-only sound rules with one shared chat-alert volume.
- Private/Talk highlighting and optional sound.
- Smart follow-latest scrolling with history preservation when old rows are trimmed.
- Smoother precision mouse-wheel scrolling and a custom dark scrollbar.
- Stable per-user sender colors so the same player is easy to recognize across messages.
- Click-through mode, compact mode, opacity, fonts, timestamps, channel colors, screen-edge collapse, and Always-on-Top support.
- The persistent Chat Overlay stays out of Windows Alt+Tab while remaining visible on-screen.
- IPv4 + IPv6 BPSR TCP capture support.
- Richer voice transcript, multilingual-notice, and hypertext extraction for filtering.
- v1.2 RC: optional English translation and Google gTTS-style speech for Guild / Party chat.
- Portable self-contained Windows x64 EXE; no .NET installation required.

## Quick start

1. Make sure **Npcap** is installed.
2. Download `BPSR-ReadyAlert.exe` from [Releases](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest).
3. Run it and select Resonance Logs CN if automatic detection misses it.
4. Leave ReadyAlert running in the system tray.
5. Enable **Chat Overlay** from the tray only if you want it.

ReadyAlert does **not** inject into BPSR, replace game files, send chat, or automate gameplay input.

## Chat Overlay

The overlay is optional. Disabling it stops the chat decoding/UI path while preserving the original Ready/Queue alert features.

### Tabs and filters

Each tab can choose channels and filter what is shown. Matching is case-insensitive.

A simple filter such as:

```text
PA
```

matches `PA` anywhere in the searchable text. For an exact whole word, use regex word boundaries:

```regex
\bPA\b
```

For several exact alternatives:

```regex
\b(?:tina|tr|towering)\b
```

This matches the whole words `tina`, `tr`, or `towering`, but not words such as `train` or `try`.

Invalid or timed-out regex fails safely instead of blocking the app.

### Sound rules

You can configure up to **3 prioritized sound rules**.

- Rule 1 has highest priority, then Rule 2, then Rule 3.
- Only the first matching rule plays for a message.
- Sound rules match **message content only** — sender/player names are intentionally ignored.
- All rules share the single **Chat alert volume** setting.
- Private/Talk dedicated sound, when enabled, takes priority over keyword rules.
- Custom notification WAV files must be standard 16-bit PCM and are limited to 4 MiB / 15 seconds to keep memory use predictable.

Example exact-word sound rule:

```regex
\b(?:mrhard|mrez)\b
```

### Speech & English translation — v1.2 RC

Open **Chat Overlay → Settings → Speech & translation**.

Translation and TTS are opt-in and have separate channel toggles:

- **Guild** maps to BPSR Union chat.
- **Party / Team** maps to BPSR Team + Group chat.
- World chat is intentionally never read aloud by this feature.
- Translation can show `↳ EN: ...` underneath the original non-English message in the overlay.
- TTS auto-detects/translates the selected message to English before speaking it. Already-English messages are spoken as-is.
- **Read sender name** optionally announces the player name before the message.
- **My BPSR username** suppresses TTS for messages sent by that exact username; matching is case-insensitive.
- **TTS volume** is independent from ReadyAlert's Ready Check / keyword-sound volume.
- Speech is queued sequentially so messages do not talk over each other; stale queued speech is dropped instead of reading very old chat.

This feature uses undocumented Google Translate / gTTS-style web endpoints and requires an internet connection, but **no Google Cloud project or API key**. Only chat selected for enabled translation/TTS processing is sent to Google. Because the endpoint is not an official compatibility contract, Google can rate-limit or change it. Translation/TTS failures are soft: ReadyAlert's local chat overlay and packet capture keep running normally.

The Google worker is dormant when both translation and TTS are disabled.

### Scrolling

When you are already following the newest chat, new messages keep the view pinned to the bottom. If you manually scroll upward, ReadyAlert preserves your reading position and shows the **new messages** control instead of snapping you down.

v1.1.1 also:

- preserves follow-latest when bounded history removes old rows;
- cancels stale wheel animations before programmatic `Go to latest` scrolling;
- supports high-resolution mouse-wheel deltas;
- respects Windows no-scroll / page-at-a-time wheel settings;
- uses an owner-drawn dark scrollbar instead of relying on inconsistent native scrollbar theming.

### Sender colors

Usernames receive deterministic contrast-safe colors. The same BPSR sender ID keeps the same color across messages and restarts, making consecutive speakers easier to distinguish. A name-based fallback is used when an ID is unavailable.

## Collapse, hide, Alt+Tab, and sounds

Collapsing or hiding the overlay affects presentation only. In v1.1.1, chat notification matching runs independently from overlay rendering, so keyword/private sound evaluation does not depend on the window being expanded, repainted, or actively draining UI rows.

Starting with v1.1.2, the persistent Chat Overlay is created as a Windows tool/overlay window, so it stays visible on-screen but is excluded from **Alt+Tab** and the taskbar. This applies while expanded, collapsed, click-through, or Always-on-Top. Settings and support dialogs remain normal windows while they are open.

In v1.2 RC, translation/TTS also runs on its own bounded background worker. Hiding or collapsing the overlay does not stop enabled Guild/Party speech processing.

Disabling **Chat Overlay** from the tray stops the chat processing path entirely.

## Privacy and architecture

ReadyAlert reads the local BPSR network stream through Npcap. It does not inject into the game process.

The Chat Overlay reuses the existing capture pipeline. It does **not** create a second Npcap capture handle, TCP reassembler, decompressor, or parallel packet-processing stack just for chat.

Parsed chat is kept in bounded local memory for the overlay. When v1.2 translation/TTS is enabled, only selected message text is sent to Google for that feature; ReadyAlert does not upload unrelated chat itself.

## Capture diagnostics

Open:

**Chat Overlay → Settings → Advanced → Chat capture status**

Use this when debugging missing chat or notification sounds. The diagnostics expose capture/parser activity and the independent notification engine status so you can distinguish capture issues, parser issues, rule mismatches, playback failures, and notification-queue drops.

## Build

The project targets **.NET 10 Windows** and publishes as a self-contained single-file x64 EXE.

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:PublishTrimmed=false -p:EnableCompressionInSingleFile=true -o dist
```

GitHub Actions enforces a **55 MiB** EXE size budget and runs the built EXE with the internal smoke/regression test suite before artifacts or releases are produced. The v1.2 smoke suite validates channel selection, own-username suppression, settings normalization, and safe TTS chunking without making network calls to Google.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
