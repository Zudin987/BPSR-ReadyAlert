# Third-party notices

## WinDivert 2.2.2

Windows release builds contain the unmodified official x64 `WinDivert.dll` and `WinDivert64.sys` from WinDivert 2.2.2-A as embedded runtime resources. They are extracted to a private, versioned directory at runtime and dynamically loaded.

WinDivert is copyright basil and is dual-licensed under the GNU Lesser General Public License v3 or GNU General Public License v2. The official WinDivert `LICENSE` file is embedded in the release and extracted beside the runtime files.

Build-time SHA-256 checks:

```text
c1e060ee19444a259b2162f8af0f3fe8c4428a1c6f694dce20de194ac8d7d9a2  WinDivert.dll
8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2  WinDivert64.sys
```

Source / releases: https://github.com/basil00/WinDivert

## ZstdSharp.Port

`ZstdSharp.Port` is used to decode zstd-compressed BPSR frames. It is licensed under the MIT License.

Source: https://github.com/oleg-st/ZstdSharp

## BPSR-ZDPS default alert sound

The default `LetsDoThis.wav` is fetched at build time from `Blue-Protocol-Source/BPSR-ZDPS` and verified against the exact SHA-256 selected for this project:

```text
0befc4c0b6a40ef374fb75c6f4c658850439ee43fa9a3c0d74d904c76627048a  LetsDoThis.wav
```

BPSR-ZDPS is published under the MIT License.

Source: https://github.com/Blue-Protocol-Source/BPSR-ZDPS

## Protocol references

Packet service/method IDs and protobuf field behavior were cross-checked against the public BPSR community projects below:

- `fudiyangjin/resonance-logs-cn` (AGPL-3.0)
- `Blue-Protocol-Source/BPSR-ZDPS` (MIT)

No files from Resonance Logs CN are modified or redistributed by this project.
