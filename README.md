# BPSR Ready Alert

**BPSR Ready Alert** is a lightweight Windows companion for **Blue Protocol: Star Resonance (BPSR)**. It keeps the original Ready Check / matchmaking sound alerts and adds an optional floating **Chat Overlay** with filters, custom tabs, keyword sounds, smoother scrolling, per-user sender colors, optional English translation, and optional Guild / Party text-to-speech.

**Website:** https://zudin987.github.io/projects/readyalert/

**Current stable release:** v1.1.2  
**Experimental branch:** v1.2.0 RC4

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
- Optional no-key **Google English (`en`)** text-to-speech for **Guild and Party / Team only**, with its own volume, sender-name toggle, own-username ignore rule, and one-click test button.
- Click-through mode, compact mode, opacity, fonts, timestamps, channel colors, screen-edge collapse, and Always-on-Top support.
- The persistent Chat Overlay stays out of Windows Alt+Tab while remaining visible on-screen.
- IPv4 + IPv6 BPSR TCP capture support.
- Richer voice transcript, multilingual-notice, and hypertext extraction for filtering.
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

### Content cleanup — v1.2 RC4

Open **Chat Overlay → Settings → Interaction**. The global cleanup options are placed together:

- **Hide sticker messages** — existing sticker filter.
- **Hide emoji-only messages** — recognizes BPSR sprite tokens from `<sprite=1>` through `<sprite=100>`, including rows containing several sprite tokens.
- **Hide linked-item / Hypertext messages** — hides parsed Hypertext chat and placeholder rows such as `[Hypertext 3000001]` or `[Hypertext 1050001] MrHard`.

The emoji filter is deliberately token-aware: a normal message such as `hello <sprite=31>` remains visible because the row is not emoji-only. Sprite-only and Hypertext placeholder messages are also excluded from TTS so ReadyAlert does not literally speak markup such as “sprite equals 31” or “Hypertext 3000001”.

### Translation and TTS — v1.2 RC4

Open **Chat Overlay → Settings → Speech & translation**.

**Translation** has three independent channel toggles:

- World
- Guild (`Union`)
- Party / Team (`Team` + `Group`)

When enabled, ReadyAlert displays the original BPSR message immediately and adds `↳ EN:` underneath when Google returns a non-English → English translation. Translation work is asynchronous and bounded so capture/UI do not wait for Google.

**Text-to-speech** intentionally has only two channel toggles:

- Guild
- Party / Team

World chat is never read aloud. TTS uses Google's no-key Translate TTS endpoint with the **English `en` voice**. Non-English messages are first translated to English when possible, then spoken by the English voice. The TTS volume is independent from Ready/keyword sounds. `Read sender name` is optional, and **My BPSR username** suppresses speech for the user's own messages using an exact case-insensitive name match.

RC2 replaced RC1's legacy Windows MCI MP3 playback with **NAudio + Windows Media Foundation**, added audio MIME validation/retry behavior inspired by the Google fallback used in the user's Discord TTS bot, and added a one-click test action. RC3 switched the Google TTS language from Malay `ms` to English `en`. RC4 keeps that audio path and adds the emoji/linked-item cleanup filters. Use **Test Google English TTS** first when checking a PC: if the test voice plays, the Google/audio backend is healthy and any remaining issue is channel/message selection rather than playback.

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

The Chat Overlay reuses the existing capture pipeline. It does **not** create a second Npcap capture handle, TCP reassembler, decompressor, or parallel packet-processing stack just for chat.

Parsed chat is kept in bounded local memory for the overlay. When optional translation/TTS is enabled, only messages selected by those channel toggles are sent to Google's Translate/gTTS web services. Sprite-only and Hypertext placeholder rows are excluded from the speech/translation pipeline.

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

GitHub Actions enforces a **55 MiB** EXE size budget and runs the built EXE with the internal smoke/regression test suite before artifacts or releases are produced.

[Latest release](https://github.com/Zudin987/BPSR-ReadyAlert/releases/latest) · [License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md)
