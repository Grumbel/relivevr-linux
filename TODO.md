# TODO / Handoff

## Status
- Logcat: `Pose for Present pts=71 not found` + `Video Decoder input is full`.
- Video `pts` was encoder wall-clock micros; protocol expects `frameNum * 16666`.
- Fixed continuous stream PTS to match protocol.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# logcat should not spam Present pts not found / decoder full
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-076.1-fix-video-pts-f993e2b.bundle`
