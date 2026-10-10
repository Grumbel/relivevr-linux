# TODO / Handoff

## Status
- Fixed FFmpeg AU reader: short-read/idle flushes were emitting 13–150B pipe
  chunks as AUs → "skip tiny AU" spam and solid green.
- Only complete VCL AUs are emitted now.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: live frame #N L=thousands R=thousands, not "skip tiny AU"
```

## Bundle
(to be produced)
