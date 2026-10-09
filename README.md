# relivevr-linux

Experimental Linux (NixOS) server for the **AMD ReliveVR** protocol
(Radeon ReLive for VR / Wireless GVR). Talks to the official Android
headset client over UDP port **1235**.

## Goal

1. Discover and connect to a ReliveVR headset client
2. Stream video the headset can decode (done for static / test patterns)
3. Receive pose and controller input
4. Later: SteamVR / OpenXR / monado, or merge useful bits into ALVR / WiVRn

## Status (2026-10-09)

| Stage | State |
|-------|--------|
| Discovery + HELLO / HELLO_DIRECT | Working |
| VideoInit → MediaCodec avc 1440×1440 | Working |
| H.264 on channel 1 (VideoData + StreamFlowCtrl) | Working |
| Stereo (frmType 0 left / 1 right) | Working |
| Per-eye independence | Confirmed (left blue / right red) |
| Complex test pattern | Confirmed (LEFT / RIGHT grids) |
| Decoder lag | ~8–11 ms when stream is fed |
| type 9 (VideoForceIDR / keepalive) | Handled; do **not** flood IDR replies |
| DeviceEvent type 4 empty `{}` | Daydream system button → `sys_click` |
| Pose / controllers | **Working** — S→C StartSensor unlocks `/hmd/pose` + `/ctrlRight/pose` |
| Button / axis `/in/` paths | **Working** — stored in `LatestPoses.inputs` |
| OpenGL pose visualizer | `RELIVEVR_VIZ=1` — grid + HMD/controller boxes + trackpad |
| SteamVR / ALVR | Future |

## Run

```bash
nix run .
# Headset on same LAN; ReliveVR app in discovery mode
```

Optional env vars:

| Variable | Effect |
|----------|--------|
| `RELIVEVR_STYLE` | Hello response style |
| `RELIVEVR_TYPE` | Hello type byte |
| `RELIVEVR_VIDEOINIT_TYPE` | VideoInit type byte |
| `RELIVEVR_START_SENSOR=1` | Experimental `CStartSensor` probe (does **not** unlock pose) |
| `RELIVEVR_VIZ=1` | OpenGL window visualizing HMD + controller poses / trackpad |

## Protocol (short)

- **Transport:** UDP (default) port 1235; 15-byte FlowCtrl fragment header (big-endian)
- **Channels:** 0 service (JSON), 1 video, 2 audio, 7 DeviceEvent
- **Video:** H.264 baseline (AVC), dual-eye, PTS in microseconds, wall-clock paced ~60 fps
- **Control:** typed JSON (`Hello`, `StartRequest`, `VideoInit`, `TrackableDeviceCaps`, …)

See `docs/protocol.md` for wire layout and `docs/re-notes.md` for RE notes.

## Pose status

Working. After VideoInit the server must send type **5**
`{"Message":"StartSensor"}` (S→C). The client then streams type **4**
DeviceEvents on channel 4 with `/hmd/pose` and `/ctrlRight/pose`
(`orient` quaternion, `pos`, batteries).

## Docs

| File | Contents |
|------|----------|
| `docs/protocol.md` | Wire format, channels, message types |
| `docs/re-notes.md` | Static RE findings (SO symbols, strings, disasm) |
| `TODO.md` | Current tip, open work, git-bundle handoff |
| `AGENTS.md` | Project rules for agents |

## Handoff

Work is delivered as **git bundles** stacking on base `6813f93`.
See `TODO.md` for the current tip bundle path.

## License

GPL-3.0-or-later
