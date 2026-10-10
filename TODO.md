# TODO / Handoff

## Status
- **PTS fix (078.1):** both `pts` and `ptsSensor` = latest client pose `time`
  (~1e16 ns-scale). Present indexes the pose queue by `pts`; frameNum alone or
  frameNum×16666 never matched. Hold video until first pose arrives so the
  decoder is not flooded with unpresentable frames.
- Crop: prefer encode at native 1440 for Daydream (`Wrong cropped rect 1440 vs 720`).
- Re-test: confirm `Pose for Present pts=… not found` and decoder-full are gone.

## Test
```bash
# Prefer native res to avoid ACodec crop mismatch
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# or 720 knowing crop warning may remain on some builds
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-078.1-pts-pose-clock-f993e2b.bundle`
