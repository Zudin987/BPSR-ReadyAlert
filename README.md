# BPSR Ready Alert

**BPSR Ready Alert** is a lightweight Windows companion for **Blue Protocol: Star Resonance (BPSR)**. It keeps the original Ready Check / matchmaking sound alerts and adds an optional floating **Chat Overlay** with filters, custom tabs, keyword sounds, smoother scrolling, per-user sender colors, optional English translation, and optional Guild / Party text-to-speech.

**Website:** https://zudin987.github.io/projects/readyalert/

**Current stable release:** v1.2.1

## Highlights

- Ready Check and matchmaking / party-confirm sound alerts.
- Optional floating Chat Overlay using the same capture pipeline — no second Npcap engine.
- World, Guild/Team, All, and custom chat tabs.
- Channel selection, minimum-level filters, Show/Hide expressions, OR/AND matching, and advanced regex.
- Optional global cleanup for sticker messages, emoji-only `<sprite=1>` through `<sprite=100>` messages, and linked-item / Hypertext messages.
- Up to 3 prioritized message-only sound rules with one shared chat-alert volume.
- Private/Talk highlighting and optional sound.
- Smart follow-latest scrolling with history preservation when old rows are trimmed.
- Smoother precision mouse-wheel scrolling and a custom dark scrollbar.
- Stable per-user sender colors so the same player is easy to recognize across messages.
- Optional no-key Google translation of **World, Guild, and Party / Team** chat to English.
- Optional no-key **Google English (`en`)** text-to-speech for **Guild and Party / Team only**, with its own volume, sender-name toggle, own-username ignore rule, one-click test button, and a toolbar TTS on/off quick toggle.
- TTS-priority scheduling keeps Guild / Party speech responsive even when World translation is busy.
- Click-through mode, compact mode, opacity, fonts, timestamps, channel colors, screen-edge collapse, and Always-on-Top support.
- The persistent Chat Overlay stays out of Windows Alt+Tab while remaining visible on-screen.
- IPv4 + IPv6 BPSR TCP capture support.
- Richer voice transcript, multilingual-notice, and hypertext extraction for filtering.
- Portable self-contained Windows x64 EXE; no .NET installation required.

## v1.2.1 — UI/UX and reliability hardening

v1.2.1 keeps the v1.2 feature set while making normal use clearer, smoother, and harder to misconfigure.

- **Three independent audio volumes are explicit:** Ready / Queue volume lives in the tray, Chat alert volume controls keyword + Private/Talk sounds, and TTS volume controls spoken Guild / Party chat only. Changing one never changes either of the others.
- Removed Windows system-sound fallbacks that could bypass ReadyAlert's configured volume after an audio-file failure.
- TTS toolbar state is clearer: green = active, red/strikethrough = off, amber = master TTS is on but muted or has no enabled speech channel.
- Settings now distinguish **Saved**, **Unsaved**, and **Applied — not saved** states. Closing warns about unapplied edits or settings that could not be persisted to disk.
- Blocking a player now consistently suppresses that player's ReadyAlert chat row, chat alert sounds, translation, and not-yet-playing TTS. Ready / Queue alerts remain unrelated.
- Clipboard actions retry transient Windows clipboard locks instead of risking an unhandled UI error.
- Adapter switching preflights the requested Npcap adapter and rolls back to the previous capture when a switch fails.
- Heavy chat bursts use bounded per-tick UI work and coalesced redraws so the overlay stays responsive.
- Capture diagnostics pause live text replacement while the user is selecting or scrolling diagnostic text.
- The single-file smoke suite includes deterministic v1.2.1 regression checks for volume separation, settings persistence states, blocked-user routing, content-cleanup sound behavior, TTS status, and bounded UI draining.

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

### Content cleanup — v1.2.0

Open **Chat Overlay → Settings → Interaction**. The global cleanup options are placed together:

- **Hide sticker messages** — existing sticker filter.
- **Hide emoji-only messages** — recognizes BPSR sprite tokens from `<sprite=1>` through `<sprite=100>`, including rows containing several sprite tokens.
- **Hide linked-item / Hypertext messages** — hides parsed Hypertext chat and placeholder rows such as `[Hypertext 3000001]` or `[Hypertext 1050001] MrHard`.

The emoji filter is deliberately token-aware: a normal message such as `hello <sprite=31>` remains visible because the row is not emoji-only. Linked-item matching is limited to actual Hypertext message kinds / `[Hypertext ...]` markers, so ordinary chat that merely uses the word “hypertext” remains visible. Sprite-only and Hypertext placeholder messages are also excluded from TTS so ReadyAlert does not literally speak markup such as “sprite equals 31” or “Hypertext 3000001”.

### Translation and TTS — v1.2.0

Open **Chat Overlay → Settings → Speech & translation**.

**Translation** has three independent channel toggles:

- World
- Guild (`Union`)
- Party / Team (`Team` + `Group`)

When enabled, ReadyAlert displays the original BPSR message immediately and adds `↳ EN:` underneath when Google returns a non-English → English translation. Translation work is asynchronous and bounded so capture/UI do not wait for Google.

**Text-to-speech** intentionally has only two channel toggles:

- Guild
- Party / Team

World chat is never read aloud. TTS uses Google's no-key Translate TTS endpoint with the **English `en` voice**. Non-English messages are first translated to English when possible, then spoken by the English voice. The TTS volume is independent from both Ready / Queue volume and Chat alert volume. `Read sender name` is optional, and **My BPSR username** suppresses speech for the user's own messages using an exact case-insensitive name match.

A compact **TTS** quick-toggle sits directly between `+ Tab` and Settings in the overlay toolbar. Green `TTS` means active. Red strikethrough `TTS` means disabled. Amber means TTS is enabled but cannot currently speak because its volume is 0% or no Guild / Party speech channel is selected. Clicking the button changes and saves the same master TTS setting used on the Speech & translation page. The quick toggle does not change the Guild / Party channel selections; it only switches the TTS master setting on or off.

During v1.2 development, playback moved from legacy Windows MCI to **NAudio + Windows Media Foundation**, Google TTS switched to the English `en` voice, emoji-only and Hypertext cleanup were added, and the toolbar TTS toggle was introduced. Final hardening gives TTS-capable Guild / Party messages priority over translation-only work, re-checks live speech eligibility before playback, bounds wake/result queues, retries appropriate Google failures, and plays long multi-chunk Google MP3 responses as independent audio chunks. Use **Test Google English TTS** first when checking a PC: if the test voice plays, the Google/audio backend is healthy and any remaining issue is channel/message selection rather than playback.

These Google Translate/gTTS-style endpoints do not require a Cloud API key, but they are undocumented and can be rate-limited or changed by Google. Failures are soft: normal ReadyAlert capture and overlay behavior continue.

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

Disabling **Chat Overlay** from the tray stops the chat processing path entirely.

## Privacy and architecture

ReadyAlert reads the local BPSR network stream through Npcap. It does not inject into the game process.

The Chat Overlay reuses the existing capture pipeline. It does **not** create a second Npcap capture handle, TCP reassembler, decompressor, or parallel packet-processing stack just for chat. Adapter switching may briefly open a validation handle before changing adapters, but only one capture engine processes packets at a time.

Parsed chat is kept in bounded local memory for the overlay. When optional translation/TTS is enabled, only messages selected by those channel toggles are sent to Google's Translate/gTTS web services.

## Capture diagnostics

Open:

**Chat Overlay → Settings → Advanced → Chat capture status**

The diagnostics show capture/parser counters, keyword/private notification counters, and translation/TTS processing/failure/drop counters. This helps distinguish packet-capture problems, parser problems, channel/TTS selection, Google failures, and audio-playback failures without guessing.

## Build

The project targets **.NET 10 Windows** and publishes as a self-contained single-file x64 EXE.

```powershell
./scripts/prepare-build-assets.ps1
dotnet publish src/BPSR.ReadyAlert/BPSR.ReadyAlert.csproj -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:PublishTrimmed=false -p:EnableCompressionInSingleFile=true -o dist
```

GitHub Actions enforces a **55 MiB** EXE size budget and runs the built EXE with the internal smoke/regression test suite before artifacts or releases are produced.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
