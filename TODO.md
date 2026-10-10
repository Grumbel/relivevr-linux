# TODO / Handoff

## Status
- FFmpeg pipe works (fflags + sliced-threads=0 fixed).
- **Solid green:** VideoInit with **0B SPS** while encoder still warming up.
  Now **defer VideoInit** until live SPS/PPS exist, then flush + arm stream.
- libx264 @ 1440² is CPU-bound — use 720² or nvenc for usable fps.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: defer message OR immediate VideoInit with non-zero param sets
# then live frames; headset should show scene (not solid green)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=nvenc cargo run
```

## Next
1. Confirm scene on HMD (not green).
2. nvenc auto path for native 1440 @ 75.

## Bundle
(to be produced)
