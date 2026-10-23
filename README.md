# relivevr-linux

Experimental Linux (NixOS) server for the **AMD ReliveVR** protocol
(Radeon ReLive for VR / Wireless GVR). Talks to the official Android
headset client (`com.amd.wirelessgvr`, Lenovo Mirage Solo / Daydream) over
UDP port **1235**.

This repository is the reverse-engineering side: protocol documentation and a
standalone test server that streams its own OpenGL scene. **To play SteamVR
games, use [alvr-relivevr](https://github.com/Grumbel/alvr-relivevr)**, an
ALVR fork built on what was learned here.

## Goal

1. Discover and connect to a ReliveVR headset client
2. Stream video the headset can decode
3. Receive pose and controller input
4. SteamVR — done through an ALVR fork (see below); later maybe OpenXR / monado / WiVRn

## Status (2026-10-14)

| Stage | State |
|-------|--------|
| Discovery + HELLO / HELLO_DIRECT | Working |
| VideoInit (avc) | Working — the Daydream client always decodes avc (HEVC shows green) |
| Live stereo encode → headset | **Working** — hardware H.264 (VAAPI) at 1440² × 75 fps, ~25 ms end-to-end |
| H.264/HEVC on channel 1 (VideoData) | Working |
| Stereo (`frmType` 0 left / 1 right) | Working — **one encoder per eye** |
| Pose / controllers | Working — S→C StartSensor unlocks poses |
| OpenGL pose visualizer | `RELIVEVR_VIZ=1` |
| Lens geometry | Working — linear 100° FOV, `NonLinearScaling=false`; GVR distorts on the headset |
| Official Windows server | Uses **HEVC**, large single UDP datagrams (~65KB) |
| SteamVR (ALVR fork) | **Working** — SteamVR Home/menu on the headset, Daydream controller as Vive wand |

## Getting the ReliveVR app

AMD no longer distributes the headset app. It's archived on APKMirror:
<https://www.apkmirror.com/apk/amd-advanced-micro-devices-inc/radeon-relive-for-vr/>. Pick **`com.amd.wirelessgvr` 1.0.13** (versionCode 13, arm64-v8a +
armeabi-v7a), the Daydream build tested here. AMD's other ReliveVR apps
(`com.amd.wirelesshmd` for Oculus, `com.amd.wirelessvive` for Vive Focus)
are untested. The APK is AMD's proprietary software and isn't redistributed
in this repository.

Check the download and install it over adb (developer mode on the headset):

```bash
sha256sum com.amd.wirelessgvr_1.0.13*.apk
# 992b0ccd9c02e5efd876e6ff0b4fe477d3d37761429069cfaf8976ad4ff62496
adb install com.amd.wirelessgvr_1.0.13*.apk
```

## Run

Headset on the same LAN, ReliveVR app open (it finds the server by
broadcast). Only one server can listen on port 1235, so stop SteamVR/ALVR
first.

| Command | What it does |
|---------|--------------|
| `nix run .#viz` | **Recommended.** OpenGL pose visualizer, streamed live to the headset (hardware H.264 at 1440² × 75 fps) |
| `nix run .` | Server without a window (test stream) |
| `nix run .#restart-client` | Restart the ReliveVR app on the headset over adb (clean reconnect) |
| `nix run .#logcat` | Follow the client's log over adb (`AMF_TRACE`) |

Default encode size is **1440×1440** (native panel). At that size software
encoders are CPU-bound; use `720` for usable fps on CPU, or hardware H.264.

### Environment variables

| Variable | Effect |
|----------|--------|
| `RELIVEVR_VIZ=1` | OpenGL pose + live stereo encode window |
| `RELIVEVR_ENCODER` | Encoder backend (see below) |
| `RELIVEVR_ENCODE_W` / `RELIVEVR_ENCODE_H` | Encode resolution (default 1440×1440) |
| `RELIVEVR_ENCODE_BITRATE` | Target bitrate in bps (default ~10–50M) |
| `RELIVEVR_ENCODE_FPS` | Encode/send rate and `pts` step (default 75 = Mirage Solo display) |
| `RELIVEVR_FOV_DEG` | Per-eye FOV (default 100°): advertised in Hello and used for rendering; GVR on the headset does the lens distortion |
| `RELIVEVR_ENCODE_GOP` | IDR interval in frames (default = fps, ≈1 s; bounds stream start delay) |
| `RELIVEVR_STYLE` | Hello response style |
| `RELIVEVR_TYPE` | Hello type byte |
| `RELIVEVR_VIDEOINIT_TYPE` | VideoInit type byte |
| `RELIVEVR_START_SENSOR=1` | Experimental CStartSensor probe |
| `RELIVEVR_VAAPI_DEVICE` | VAAPI render node (default `/dev/dri/renderD128`) |

### `RELIVEVR_ENCODER`

| Value | Backend |
|-------|---------|
| `auto` (default) | FFmpeg h264_nvenc → h264_vaapi → h264_qsv → libx264 → OpenH264 |
| `hevc` / `x265` / `h265` | HEVC: hevc_nvenc → hevc_vaapi → libx265. **Green on Daydream**: the client keeps its AVC decoder |
| `hevc_vaapi` / `hevc_nvenc` | HEVC on that API only |
| `openh264` / `soft` | Cisco OpenH264 (reliable AVC path) |
| `x264` / `libx264` | FFmpeg libx264 ultrafast / zerolatency |
| `nvenc` | FFmpeg h264_nvenc |
| `vaapi` | FFmpeg h264_vaapi |
| `qsv` | FFmpeg h264_qsv |
| `ffmpeg` | FFmpeg only (no OpenH264 fallback) |

Hardware and libx264/libx265 use the **ffmpeg CLI** (pipes), not linked libavcodec.

## SteamVR (ALVR fork)

SteamVR streaming lives in the ALVR fork
[alvr-relivevr](https://github.com/Grumbel/alvr-relivevr) (branch `master`,
on ALVR master's direct-mode API; the earlier v20.14.1-based attempt is
branch `relivevr-20.14.1`). `alvr/server_core/src/relivevr.rs` replaces
ALVR's client protocol. Setup, flake apps and its open issues are in that
repository's README and TODO.md.

- Two VAAPI encoders crop each half of the side-by-side frame, shared IDR decisions.
- Server-side warp to the newest HMD pose; `ptsSensor` kept monotonic (the
  client drops poses older than the last one used).
- Daydream controller emulated as a Vive wand (touchpad, trigger, menu; long
  App press → system).
- vrcompositor's linear frames are sRGB-encoded before encoding (else the
  environment and dashboard are far too dark).
- **StartSensor is sent exactly once** — a second one stalls Present on frame 0.

How it got there (NixOS pitfalls such as host glibc and the bubblewrap
socket namespace): [docs/re-notes.md](docs/re-notes.md) "SteamVR via ALVR
fork".

## Architecture notes

- **Dual encoders**: left and right eyes use separate encoder instances. A single
  shared encoder makes the right eye a P-frame predicted from the left image
  (severe garbling).
- **VideoInit** trailer is Annex-B SPS+PPS (AVC) or VPS+SPS+PPS (HEVC), taken from
  encoder warm-up / seed. CodecID follows the live encoder (`avc` or `hevc`).
- **UDP**: one FlowCtrl datagram per eye frame, up to ~65KB (same as Windows;
  IP-fragmented on the wire). Do not split into small FlowCtrl fragments.
- **Sequence**: video `frame_seq` continues after VideoInit so the client does not
  drop early frames as `Message is old`.
- Protocol details: [docs/protocol.md](docs/protocol.md). Windows dump (dumpsmall):
  [docs/windows-pcap.md](docs/windows-pcap.md). Reverse-engineering log:
  [docs/re-notes.md](docs/re-notes.md).

## Known issues

| Symptom | Likely cause / mitigation |
|---------|---------------------------|
| Solid **green** on HMD | HEVC sent to the Daydream client (it always decodes `avc`), or a P-frame chain broken by skipped frames. Use H.264 (default). See docs/re-notes.md "Green screen with hardware encode". |
| `Wrong cropped rect … (1440) vs frame (720)` | Encode size must match VideoInit; Daydream StartRequest is 1440² — prefer native 1440 or accept crop warnings at 720. |
| `Pose for Present pts=… not found` | Windows uses `pts=frameNum*166666`, `ptsSensor`=pose time. Brief pts=0 at start is normal; sustained miss → check pose + IDR arm. |
| **Black** screen | No SPS yet / empty VideoInit; seed/warm-up should fix. Check `live video … param sets`. |
| **Garbled** then clears | Early P-frames or IDR recovery after motion; dual-encoder path. |
| Low desktop fps | Encode on worker thread; if still slow, lower `RELIVEVR_ENCODE_W/H` or use hw encode. |
| `libx264` green while OpenH264 works | MediaCodec pickier about FFmpeg H.264; use hardware H.264 (default) or `openh264`. |

## Development

See [AGENTS.md](AGENTS.md) (agent rules, headset test workflow) and
[TODO.md](TODO.md) (status, open items).

```bash
nix develop
RELIVEVR_VIZ=1 cargo run --release
```

The dev shell also has the RE tools (radare2, tshark, scapy). For RE work, put the APK
in `third_party/` (git-ignored); see
[Getting the ReliveVR app](#getting-the-relivevr-app).
