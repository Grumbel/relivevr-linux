# TODO / Handoff

## Status
- Stereo HMD-camera live encode (IPD 64 mm) working.
- **Hardware encode path** via FFmpeg (nvenc / vaapi / qsv / libx264), OpenH264 software fallback.
  Select with `RELIVEVR_ENCODER=auto|nvenc|vaapi|qsv|x264|openh264|ffmpeg`.
  Default `auto`: try nvenc → vaapi → qsv → libx264 → OpenH264.
- Encode resolution configurable (default **720×720**, target **~75 Hz**).
  `RELIVEVR_ENCODE_W/H`, `RELIVEVR_ENCODE_FPS`, `RELIVEVR_ENCODE_BITRATE`.
- FOV adjustable via Daydream **vol+/-** (default **90°**); near 0.08 m.
- Checkerboard room; pose / trackpad / buttons visualized.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# force NVIDIA:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=nvenc cargo run
# force VA-API (AMD/Intel):
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=vaapi cargo run
# software only:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# full native res once HW is confirmed:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=auto RELIVEVR_ENCODE_W=1440 RELIVEVR_ENCODE_H=1440 cargo run
```

## Next
1. Confirm HW path on real GPU (check logs for "encoder backend: FFmpeg h264_…").
2. Raise default res/fps once HW is stable; tune bitrate.
3. Radial distortion for Daydream lenses (coeffs in docs/re-notes.md).
4. OpenXR / monado stub.

## Bundle
(to be produced)
