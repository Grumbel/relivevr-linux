# TODO / Handoff

## Status
- Stereo live encode defaults **1440² @ 75 Hz**, **10 Mbps/eye**.
- FFmpeg HW encode (nvenc/vaapi/qsv/x264) + OpenH264 fallback.
- NVENC warm-up + main profile; stderr on failure; empty live → baked patterns.
- Compile fix: E0502 in encode_rgba borrow paths.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
RELIVEVR_VIZ=1 cargo run   # auto (nvenc → …)
```

## Next
1. Confirm live scene on HMD (not baked grids).
2. Radial distortion for Daydream lenses.
3. OpenXR / monado stub.

## Bundle
`/home/workdir/artifacts/relivevr-linux-056.2-fix-encode-borrow-f993e2b.bundle`
