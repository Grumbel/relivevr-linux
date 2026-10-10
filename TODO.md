# TODO / Handoff

## Status
- Logcat: `Wrong cropped rect 1440 vs frame 720`, `Present pts=0 not found`,
  `input frame sensor pts not found`, decoder full.
- pts = frameNum; ptsSensor = latest pose `time`.
- Crop: prefer encode at native 1440 for Daydream.

## Test
```bash
# Prefer native res to avoid ACodec crop mismatch
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# or 720 knowing crop warning may remain on some builds
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-077.1-pts-pose-time-f993e2b.bundle`
