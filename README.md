# relivevr-linux

Experimental Linux (NixOS) server for the **AMD ReliveVR** protocol
(Radeon ReLive for VR / Wireless GVR). Talks to the official Android
headset client over UDP port **1235**.

## Goal

1. Discover and connect to a ReliveVR headset client
2. Stream video the headset can decode
3. Receive pose and controller input
4. Later: SteamVR / OpenXR / monado, or merge useful bits into ALVR / WiVRn

## Status (2026-10-10)

| Stage | State |
|-------|--------|
| Discovery + HELLO / HELLO_DIRECT | Working |
| VideoInit (avc / **hevc**) | Working — CodecID must match the encoder |
| Live stereo encode → headset | **Working** with OpenH264; HEVC path added |
| H.264/HEVC on channel 1 (VideoData) | Working |
| Stereo (`frmType` 0 left / 1 right) | Working — **one encoder per eye** |
| Pose / controllers | Working — S→C StartSensor unlocks poses |
| OpenGL pose visualizer | `RELIVEVR_VIZ=1` |
| Official Windows server | Uses **HEVC**, large single UDP datagrams (~65KB) |
| SteamVR / ALVR | Future |

### Recommended run (picture on Daydream)

```bash
# Software H.264 — reliable picture (may soft-garble for a moment after connect)
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run

# HEVC — matches Windows native path (needs ffmpeg with libx265 and/or hevc_nvenc/vaapi)
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

Default encode size is **1440×1440** (native panel). At that size software
encoders are CPU-bound; use `720` for usable fps on CPU, or hardware HEVC/H.264.

## Run

```bash
nix run .
# Headset on same LAN; ReliveVR app in discovery mode
```

### Environment variables

| Variable | Effect |
|----------|--------|
| `RELIVEVR_VIZ=1` | OpenGL pose + live stereo encode window |
| `RELIVEVR_ENCODER` | Encoder backend (see below) |
| `RELIVEVR_ENCODE_W` / `RELIVEVR_ENCODE_H` | Encode resolution (default 1440×1440) |
| `RELIVEVR_ENCODE_BITRATE` | Target bitrate in bps (default ~10–50M) |
| `RELIVEVR_ENCODE_FPS` | Encode fps cap (default from client or ~75) |
| `RELIVEVR_STYLE` | Hello response style |
| `RELIVEVR_TYPE` | Hello type byte |
| `RELIVEVR_VIDEOINIT_TYPE` | VideoInit type byte |
| `RELIVEVR_START_SENSOR=1` | Experimental CStartSensor probe |
| `RELIVEVR_VAAPI_DEVICE` | VAAPI render node (default `/dev/dri/renderD128`) |

### `RELIVEVR_ENCODER`

| Value | Backend |
|-------|---------|
| `auto` (default) | FFmpeg h264_nvenc → h264_vaapi → h264_qsv → libx264 → OpenH264 |
| `hevc` / `x265` / `h265` | **HEVC**: hevc_nvenc → hevc_vaapi → libx265 (Windows-native CodecID) |
| `openh264` / `soft` | Cisco OpenH264 (reliable AVC path) |
| `x264` / `libx264` | FFmpeg libx264 ultrafast / zerolatency |
| `nvenc` | FFmpeg h264_nvenc |
| `vaapi` | FFmpeg h264_vaapi |
| `qsv` | FFmpeg h264_qsv |
| `ffmpeg` | FFmpeg only (no OpenH264 fallback) |

Hardware and libx264/libx265 use the **ffmpeg CLI** (pipes), not linked libavcodec.

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
- Protocol details: [docs/protocol.md](docs/protocol.md). Reverse-engineering log:
  [docs/re-notes.md](docs/re-notes.md).

## Known issues

| Symptom | Likely cause / mitigation |
|---------|---------------------------|
| Solid **green** on HMD | Wrong codec vs VideoInit, broken AU framing, or (historically) 1400B FlowCtrl splits / shared-encoder SPS mismatch. Prefer `hevc` or `openh264`. |
| **Black** screen | No SPS yet / empty VideoInit; seed/warm-up should fix. Check `live video … param sets`. |
| **Garbled** then clears | Early P-frames or IDR recovery after motion; dual-encoder path. |
| Low desktop fps | Encode on worker thread; if still slow, lower `RELIVEVR_ENCODE_W/H` or use hw encode. |
| `libx264` green while OpenH264 works | MediaCodec pickier about FFmpeg H.264; use `hevc` or `openh264`. |

## Development

See [AGENTS.md](AGENTS.md) and [TODO.md](TODO.md) for agent handoff rules
(git bundles only, tip tracking).

```bash
cargo build
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```
