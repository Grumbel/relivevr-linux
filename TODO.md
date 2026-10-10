# TODO / Handoff

## Status
- Stereo HMD-camera live encode (IPD 64 mm); defaults **1440² @ 75 Hz**, **10 Mbps/eye**.
- Hardware encode via FFmpeg (nvenc/vaapi/qsv/x264) + OpenH264 fallback.
- **Bugfix (black screen / frozen viz):**
  - FFmpeg stdout read no longer blocks the GL thread (dedicated non-blocking AU reader).
  - Empty live slot falls through to baked LEFT/RIGHT test patterns (headset not left black).
- FOV vol+/- (default 90°); checkerboard room; pose on stdout.

## Test
```bash
RELIVEVR_VIZ=1 cargo run
# Desktop window should keep animating with pose; headset should show either
# live scene or baked LEFT/RIGHT grids (not pure black).
# Log should show encoder backend; encode errors rate-limited.

# If FFmpeg misbehaves, force software at lower res:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Next
1. Confirm live encode + 1440²@75 on real GPU.
2. Radial distortion for Daydream lenses.
3. OpenXR / monado stub.

## Bundle
`/home/workdir/artifacts/relivevr-linux-055.1-fix-ffmpeg-hang-black-f993e2b.bundle`
