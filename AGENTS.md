# ReliveVR Linux / Protocol RE Project

Reverse engineering of AMD's ReliveVR protocol plus a standalone Linux test
server. SteamVR support was built on top of it as a separate repository, the
ALVR fork [alvr-relivevr](https://github.com/Grumbel/alvr-relivevr) (its own
AGENTS.md). Protocol findings from either side are documented here.

## Goal

Reverse-engineer the AMD ReliveVR (Wireless GVR / Radeon ReLive for VR)
protocol so a Linux (NixOS) application can:

1. Discover / connect to a ReliveVR client (headset Android app).
2. Stream video the headset can decode.
3. Receive pose / controller input.
4. SteamVR: done in the ALVR fork. Maybe OpenXR / Monado / WiVRn later.

## Standing rules

- Keep the project continuable from **TODO.md + this repo alone**: update
  TODO.md (status, open items) when something significant changes.
- Document every non-obvious finding in `docs/protocol.md` and
  `docs/re-notes.md` (with evidence: logcat line, address, capture).
- Prefer a pure ReliveVR-compatible server first; evaluate ALVR/WiVRn later.
- Commit normally to `master`. Author: Ingo Ruhnke \<grumbel@gmail.com\>;
  trailer `Co-Authored-By: <model name> <noreply@…>` naming the model that
  did the work.
- License: GPL-3.0-or-later.

## Architecture (summary)

- **PC (server):** closed-source Windows OpenVR driver `driver_amdwvr.dll`
  + AMF encoder (no official Linux server). It renders a plain symmetric 100°
  image per eye; `ComputeDistortion` is a passthrough.
- **Headset (client):** Android app `com.amd.wirelessgvr` +
  `libwirelessvr-lib.so` (`awvr`). GVR on the headset does lens distortion
  and reprojection, using the FOV from our HelloResponse.
- **Transport:** UDP port **1235**, FlowCtrl 15-byte fragment header (BE),
  then type byte + JSON for control; video on channel 1 as
  `[1][VideoData JSON\0][Annex-B]`.
- **Input:** pose / controllers as JSON DeviceEvents after S→C StartSensor.

## Live status

- Hardware **H.264** (`h264_vaapi` on AMD) at 1440² per eye, 75 fps,
  ~25 ms end-to-end per the client's own stats. Run:
  `RELIVEVR_VIZ=1 cargo run --release` (inside `nix develop` for GL libs).
- The client always decodes **avc** (its StartRequest choice) and ignores
  VideoInit `CodecID` — HEVC gives a green screen until that is solved.
- VideoInit must say `NonLinearScaling:false`; rendered FOV must equal the
  Hello `HorizontalFOV`/`VerticalFOV` (`RELIVEVR_FOV_DEG`, default 100).
- Video frames are sent in order; a stream (re)starts only on an IDR.
- Pose / controllers / OpenGL preview window working.
- Run it with `nix run .#viz`. `nix run .#restart-client` and
  `nix run .#logcat` are the adb helpers (see README).

## Testing against the headset

The Mirage Solo is reachable via adb (`adb devices`; serial was `HAD1E8GL`).

```bash
# restart the client so it reconnects cleanly to a fresh server
adb shell am force-stop com.amd.wirelessgvr && sleep 1 && \
  adb shell am start -n com.amd.wirelessgvr/.MainActivity
adb logcat -c; adb logcat -v time | grep AMF_TRACE
```

Useful client lines:

- `[DisplayPipeline] Average latency: full …, decoder …, dec_subm_interval …`
  — health summary; `dec_subm_interval` should match the send rate
  (13.33 ms at 75 fps) and `in_decoder` must not grow.
- `Decoder lag`, `Video Decoder input is full`, `Pose for Present … not found`
  — pacing / pts / ptsSensor problems.
- `[MediaCodecDecoder] Init(avc)`, `VideoInit: … nonLinearScale=…`,
  `SetFOV old=… new=…` — what the client actually configured.

The client also sends type-8 JSON latency stats to the server (logged as
`unhandled type 8`).

A Windows install with the official server may be mounted at
`/net/i7/C:` (driver in `Program Files/AMD/WVR/OpenVR`, logs in
`Program Files (x86)/Steam/logs/AMDWirelessVR.log`, UTF-16).

## Key docs

| File | Contents |
|------|----------|
| `docs/protocol.md` | Wire format, live packets, message catalogue |
| `docs/windows-pcap.md` | dumpsmall.pcapng truths (VideoInit, VideoData, PTS) |
| `docs/client-apk.md` | APK RE: Present pts→sensor→pose exact lookup |
| `docs/lessons-2026-10-10.md` | Earlier session lessons (historical "tip" numbering) |
| `docs/re-notes.md` | RE log incl. green-screen, pacing and lens root causes |
| `TODO.md` | Current status and open items |
| `src/main.rs` | UDP server, protocol, video send loop |
| `src/encode.rs` | ffmpeg / OpenH264 encoders, AU framing, frame queue |
| `src/viz.rs` | OpenGL scene, GPU NV12 conversion, encode threads |
| `src/pose.rs` | DeviceEvent / pose parsing |

## Source material

- APK: `com.amd.wirelessgvr` 1.0.13 (Daydream / GVR client), in the parent
  directory.
- Related packages exist for Oculus (`com.amd.wirelesshmd`) and Vive Focus
  (`com.amd.wirelessvive`) — not analyzed in this tree.
