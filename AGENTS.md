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
- **OpenH264** live stereo works on Daydream (use `RELIVEVR_ENCODE_W/H=720` for CPU).
- **HEVC** path (`RELIVEVR_ENCODER=hevc`): hevc_nvenc → hevc_vaapi → libx265; VideoInit `CodecID=hevc` (Windows native).
- Dual per-eye encoders (shared encoder caused right-eye P-frame garbling).
- Large single UDP video datagrams (~65KB max, like Windows); no small FlowCtrl splits.
- Default encode **1440²**; software is CPU-bound there — prefer 720² or hardware.
- Pose / controllers / `RELIVEVR_VIZ=1` OpenGL window working.
- FOV via Daydream vol+/-; type-9 = keepalive (no IDR flood).

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
