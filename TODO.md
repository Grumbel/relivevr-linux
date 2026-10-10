# TODO / Handoff

## Status
- Live encode; no baked IDRs mixed with live VideoInit.
- FFmpeg pipe hardened: **4 MiB pipe buffers** (`F_SETPIPE_SZ`), **poll + idle
  AU flush**, one-frame warm-up retries.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect quick: FFmpeg libx264 … (warm-up OK, AU NNNB try 0)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
RELIVEVR_VIZ=1 cargo run
```

## Next
1. Confirm live scene on HMD (x264 and/or nvenc).
2. Radial distortion / OpenXR.

## Bundle
`/home/workdir/artifacts/relivevr-linux-060.1-ffmpeg-pipe-poll-f993e2b.bundle`
