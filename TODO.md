# TODO / Handoff

## Status
- Stereo live encode; OpenH264 confirmed; FFmpeg pipe (fps_mode on output side).
- **Do not mix baked 1440² IDRs with live VideoInit** — stream arm / continuous
  path use live NALs when a live encoder is present; baked patterns only if
  `RELIVEVR_VIZ` is off (no live slot).

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# warm-up OK, then "VideoFrame live IDR" (not 20736B baked)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# VideoInit 720²; continuous live frames (no LEFT/RIGHT baked grids)

RELIVEVR_VIZ=1 cargo run
```

## Next
1. Confirm live scene on HMD.
2. Radial distortion for Daydream lenses.
3. OpenXR / monado stub.

## Bundle
`/home/workdir/artifacts/relivevr-linux-058.1-live-not-baked-f993e2b.bundle`
