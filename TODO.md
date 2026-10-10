# TODO / Handoff

## Status
- Stereo live encode; OpenH264 path confirmed working (SPS/PPS + stream).
- FFmpeg pipe fixed: **blocking AU reader**, **fps_mode passthrough**, 3-frame warm-up,
  flush_packets (x264/nvenc were timing out with empty stderr).

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect: FFmpeg libx264 … (warm-up OK)

RELIVEVR_VIZ=1 cargo run
# auto nvenc → …

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Next
1. Confirm live scene on HMD (not baked grids).
2. Radial distortion for Daydream lenses.
3. OpenXR / monado stub.

## Bundle
(to be produced)
