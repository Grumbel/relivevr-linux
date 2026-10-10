# TODO / Handoff

## Status
- OpenH264 shows picture but was **garbled** — single shared encoder made right-eye
  P-frames predict from left-eye image. **Dual per-eye encoders** restored.
- Large single UDP (Windows-style) kept from 070.1.
- x264 still green — separate issue (FFmpeg bitstream / HEVC preference).

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: recognizable scene, not garbled

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Next
- x264 green → compare Annex-B / try HEVC
- force_idr actually wired for Soft/FFmpeg

## Bundle
`/home/workdir/artifacts/relivevr-linux-071.1-dual-encoder-no-cross-pred-f993e2b.bundle`
