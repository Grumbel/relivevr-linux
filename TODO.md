# TODO / Handoff

## Status
- FFmpeg is the **CLI tool**, not libavcodec.
- Pipe root causes fixed:
  1. `-fflags +nobuffer+flush_packets` → zero frames
  2. **`-x264-params sliced-threads=0`** → no AU while stdin open
- Rely on `-tune zerolatency` only for x264 low-latency.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect <1s: FFmpeg libx264 … (warm-up OK, AU NNNB try 0)
# NOT oneshot mode
```

## Next
1. Confirm live HMD video at usable fps.
2. Prefer nvenc/vaapi in auto once pipe is solid.

## Bundle
(to be produced)
