# Third-party notices

## Npcap

BPSR Ready Alert uses the user's existing **Npcap** installation through the standard `wpcap.dll` API for passive packet capture.

Npcap is an external runtime dependency. BPSR Ready Alert does **not** bundle, redistribute, install, modify, or update Npcap files.

Official site: https://npcap.com/

## ZstdSharp.Port

`ZstdSharp.Port` is used to decode zstd-compressed BPSR frames. It is licensed under the MIT License.

Source: https://github.com/oleg-st/ZstdSharp

## Alert sound

The bundled `LetsDoThis.wav` is reconstructed at build time from the exact user-supplied audio selected for this project. It is not downloaded from Resonance Logs CN.

## BPSR community references

`Blue-Protocol-Source/BPSR-ZDPS` (MIT) was used as a behavioral and protocol reference for Ready/Queue handling and BPSR chat service/protobuf definitions.

Source: https://github.com/Blue-Protocol-Source/BPSR-ZDPS

`kanomari/BPSR-Chat-Overlay` (MIT) was reviewed as a UI/UX behavior reference for conventional game-overlay features such as click-through, global hotkeys, Smart Scroll, screen-edge collapse, visual customization, notification highlighting and robust settings recovery. ReadyAlert keeps its own WinForms implementation and shared `CaptureEngine` architecture rather than adopting that project's independent capture stack.

Source: https://github.com/kanomari/BPSR-Chat-Overlay

## Protocol references

Packet service/method IDs, Npcap behavior, network-adapter selection behavior, and protobuf field behavior were cross-checked against public BPSR community projects including:

- `fudiyangjin/resonance-logs-cn` (AGPL-3.0)
- `Blue-Protocol-Source/BPSR-ZDPS` (MIT)

No files from Resonance Logs CN are modified or redistributed by this project.
