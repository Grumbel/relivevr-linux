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
- Stereo live H.264 from HMD pose (IPD 64 mm); default encode **1440² @ ~75 Hz**, 10 Mbps/eye (HW via FFmpeg; OpenH264 fallback).
- Hardware encode via FFmpeg (`RELIVEVR_ENCODER=auto|nvenc|vaapi|qsv|x264`) with OpenH264 fallback.
- FOV adjustable via Daydream vol+/- (default 70°, Hello advertises ~100°); checkerboard room.
- Decoder lag ~8–11 ms; type-9 treated as keepalive (no IDR flood).
- Empty type-4 DeviceEvent `{}` = Daydream system / app button (counted as `sys_click`).
- Pose working: after VideoInit the server sends type-5 `{"Message":"StartSensor"}`
  (S→C); client streams `/hmd/pose` + `/ctrlRight/pose` on channel 4.
- Button / axis DeviceEvents with `/in/` paths stored in `LatestPoses.inputs`.
- Next: Daydream lens FOV/distortion match, higher-res/GPU encode, OpenXR/monado stub.

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
