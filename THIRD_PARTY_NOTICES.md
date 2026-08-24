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

BPSR-ZDPS was used as a behavioral reference for the notification feature and packet-state handling.

Source: https://github.com/Blue-Protocol-Source/BPSR-ZDPS

## Protocol references

Packet service/method IDs, Npcap behavior, network-adapter selection behavior, and protobuf field behavior were cross-checked against the public BPSR community projects below:

- `fudiyangjin/resonance-logs-cn` (AGPL-3.0)
- `Blue-Protocol-Source/BPSR-ZDPS` (MIT)

No files from Resonance Logs CN are modified or redistributed by this project.
