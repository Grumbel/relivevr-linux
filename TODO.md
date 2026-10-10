# TODO / Handoff

## Status
- Stereo live encode defaults **1440² @ 75 Hz**, **10 Mbps/eye**.
- FFmpeg HW encode (nvenc/vaapi/qsv/x264) + OpenH264 fallback.
- **NVENC fix:** warm-up frame at spawn (fail → auto tries next backend);
  main profile (not baseline); stderr captured into error messages;
  no `-nostdin` (was risking pipe starvation).
- Empty live slot still falls through to baked LEFT/RIGHT patterns.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Expect either:
#   FFmpeg h264_nvenc … (warm-up OK)
# or auto fall-through:
#   FFmpeg h264_nvenc warm-up failed: … stderr: …
#   encoder backend: FFmpeg libx264 / OpenH264
# Headset should leave LEFT/RIGHT grids once live AUs flow.

# Force software:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# Force x264 if nvenc still dies:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
```

## Next
1. Confirm live scene on HMD (not just baked grids).
2. Radial distortion for Daydream lenses.
3. OpenXR / monado stub.

## Bundle
(to be produced)
