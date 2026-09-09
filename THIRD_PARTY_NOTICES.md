# Third-party notices

## Npcap

BPSR Ready Alert uses the user's existing **Npcap** installation through the standard `wpcap.dll` API for passive packet capture.

Npcap is an external runtime dependency. BPSR Ready Alert does **not** bundle, redistribute, install, modify, or update Npcap files.

Official site: https://npcap.com/

## ZstdSharp.Port

`ZstdSharp.Port` is used to decode zstd-compressed BPSR frames. It is licensed under the MIT License.

Source: https://github.com/oleg-st/ZstdSharp

## NAudio

`NAudio` is used in v1.2+ for reliable Windows playback of optional Google TTS MP3 audio through Windows Media Foundation / WaveOut instead of the legacy MCI backend. NAudio is licensed under the MIT License.

Source: https://github.com/naudio/NAudio

## Alert sound

The bundled `LetsDoThis.wav` is reconstructed at build time from the exact user-supplied audio selected for this project. It is not downloaded from Resonance Logs CN.

## BPSR community references

`Blue-Protocol-Source/BPSR-ZDPS` is licensed under the MIT License (Copyright (c) 2025 Blue-Protocol-Source) and was used as a behavioral/protocol reference for Ready/Queue handling and BPSR chat service/protobuf definitions. ReadyAlert also generates its complete player-facing English skill ID/name catalog at build time from ZDPS `Data/SkillOverrides.en.json`, pinned to commit `cfeb58c0acc85bc17181b413b9e50b0b26c15c5d`. The generated catalog is used only as metadata for resolving observed in-game skill IDs; unknown/future IDs remain visible numerically.

MIT permission notice: Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the conditions in the upstream MIT license.

Source: https://github.com/Blue-Protocol-Source/BPSR-ZDPS

`kanomari/BPSR-Chat-Overlay` (MIT) was reviewed as a UI/UX behavior reference for conventional game-overlay features such as click-through, global hotkeys, Smart Scroll, screen-edge collapse, visual customization, notification highlighting and robust settings recovery. ReadyAlert keeps its own WinForms implementation and shared `CaptureEngine` architecture rather than adopting that project's independent capture stack.

Source: https://github.com/kanomari/BPSR-Chat-Overlay

## Protocol references

Packet service/method IDs, Npcap behavior, network-adapter selection behavior, and protobuf field behavior were cross-checked against public BPSR community projects including:

- `fudiyangjin/resonance-logs-cn` (AGPL-3.0)
- `Blue-Protocol-Source/BPSR-ZDPS` (MIT)

No files from Resonance Logs CN are modified or redistributed by this project.
