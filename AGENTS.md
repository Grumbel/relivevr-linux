# ReliveVR Linux / Protocol RE Project

## Goal

Reverse-engineer the AMD ReliveVR (Wireless GVR / Radeon ReLive for VR)
protocol so a Linux (NixOS) application can:

1. Discover / connect to a ReliveVR client (headset Android app).
2. Send video frames (initially a static image) the headset can decode.
3. Receive pose / controller input.
4. Later: SteamVR / OpenXR / monado integration, or merge into ALVR / WiVRn.

## Standing rules

- Continuable from **TODO.md + this repo alone** (git-bundle handover).
- Document every non-obvious finding in `docs/protocol.md` and
  `docs/re-notes.md`.
- Prefer a pure ReliveVR-compatible server first; evaluate ALVR/WiVRn later.
- Author: Ingo Ruhnke \<grumbel@gmail.com\>; trailer `Co-authored-by: Grok <grok@x.ai>`.
- License: GPL-3.0-or-later.

## Architecture (summary)

- **PC (server):** closed-source Windows OpenVR driver + AMF encoder
  (no official Linux server).
- **Headset (client):** Android app + `libwirelessvr-lib.so` (`awvr`).
- **Transport:** UDP (default) or TCP, port **1235**, FlowCtrl 15-byte
  fragment header (BE), then type-byte + JSON for control.
- **Video:** MediaCodec decode (HEVC hinted); binary path separate from JSON.
- **Input:** pose / controllers back on typed channels (Channel = 0..7).

## Live status (as of tip documented in TODO.md)

- Fragment header and HelloRequest fully known from live VR-1541F traffic.
- Probe replies to discovery; client parses reply then dies or rediscovers.
- HelloResponse shape still being bisected (`RELIVEVR_STYLE` / `RELIVEVR_TYPE`).
- StartRequest / Video* JSON keys known from static RE; not yet on the wire.
- Binary video framing and Channel role map still open.

## Key docs

| File | Contents |
|------|----------|
| `docs/protocol.md` | Wire format, live packets, message catalogue |
| `docs/re-notes.md` | RE methods, addresses, open questions, probe usage |
| `TODO.md` | Current tip, next steps, bundle name |
| `src/main.rs` | UDP probe + discovery responder |

## Source material

- APK: `com.amd.wirelessgvr` 1.0.13 (Daydream / GVR client).
- Related packages exist for Oculus (`com.amd.wirelesshmd`) and Vive Focus
  (`com.amd.wirelessvive`) — not yet analyzed in this tree.
