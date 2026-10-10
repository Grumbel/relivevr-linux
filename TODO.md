# TODO / Handoff

## Status
- OpenH264: picture works (dual encoder).
- **HEVC path added** (`RELIVEVR_ENCODER=hevc`): tries hevc_nvenc → hevc_vaapi → libx265.
  VideoInit CodecID=`hevc`, param sets = VPS+SPS+PPS (Windows native).

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: FFmpeg hevc_* or libx265, VideoInit CodecID hevc, non-green

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
```

## Bundle
(to be produced)
