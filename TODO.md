# TODO / Handoff

## Status
- Live encode path; baked 1440² IDRs not mixed with live VideoInit.
- FFmpeg pipe: **one-frame warm-up** (avoid stdin/stdout pipe deadlock on
  large NV12 frames), unbounded AU channel, fps_mode on output side.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect: FFmpeg libx264 … (warm-up OK, AU NNNB try 0)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
RELIVEVR_VIZ=1 cargo run
```

## Next
1. Confirm live scene on HMD.
2. Radial distortion / OpenXR.

## Bundle
`/home/workdir/artifacts/relivevr-linux-059.1-fix-ffmpeg-pipe-deadlock-f993e2b.bundle`
