# TODO / Handoff

## Status
- Stereo HMD-camera live encode (IPD 64 mm) working.
- **Hardware encode** via FFmpeg (nvenc / vaapi / qsv / libx264), OpenH264 fallback.
  `RELIVEVR_ENCODER=auto|nvenc|vaapi|qsv|x264|openh264|ffmpeg`.
- Defaults locked for native headset rate:
  - **1440×1440** per eye (client StartRequest size)
  - **~75 Hz** (`RELIVEVR_ENCODE_FPS`)
  - **10 Mbps/eye** bitrate (`RELIVEVR_ENCODE_BITRATE`)
  - OpenH264 at this size warns; lower with `RELIVEVR_ENCODE_W/H=720` if on software.
- FOV adjustable via Daydream **vol+/-** (default **90°**); near 0.08 m.
- Checkerboard room; pose / trackpad / buttons visualized.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# expect log: encoder backend: FFmpeg h264_…  and  Live stereo encode FBO 1440x1440 target 75 fps

# software-friendly:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run

# more quality:
RELIVEVR_VIZ=1 RELIVEVR_ENCODE_BITRATE=20000000 cargo run
```

## Next
1. Confirm HW path + 1440²@75 on real GPU (bitrate / blockiness tuning).
2. Radial distortion for Daydream lenses (coeffs in docs/re-notes.md).
3. OpenXR / monado stub.

## Bundle
(to be produced)
