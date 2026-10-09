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

- Discovery + HELLO_DIRECT + StartRequest + VideoInit working.
- H.264 (AVC) 1440×1440 dual-eye stream; LEFT/RIGHT grid patterns confirmed.
- Decoder lag ~8–11 ms; type-9 treated as keepalive (no IDR flood).
- Empty type-4 DeviceEvent `{}` = Daydream button, not pose.
- Pose blocked: client SensorThread / QueryAndSendSensors never active.
- Next: Windows packet capture or on-device Frida of SetActive.

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
