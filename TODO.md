# TODO / Handoff

## Status
- FFmpeg persistent pipe still fails warm-up on some hosts (AU timeout, empty stderr).
- **Oneshot fallback**: if pipe warm-up fails, each frame is encoded via a fresh
  `ffmpeg -frames:v 1` (stdin close → read_to_end). Slower but reliable.
- Concurrent warm-up write thread; idle-flush any buffered AU data; annexb=1.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect either:
#   FFmpeg libx264 … (warm-up OK, AU …)
# or:
#   persistent pipe warm-up failed … trying oneshot mode
#   FFmpeg libx264 oneshot mode ready … (probe AU …B)
# then: live video SPS/PPS …B

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Next
1. Confirm live video on HMD (oneshot x264 or openh264).
2. Investigate why persistent pipe stays silent (host ffmpeg build?).
3. force_idr / distortion / OpenXR.

## Bundle
(to be produced)
