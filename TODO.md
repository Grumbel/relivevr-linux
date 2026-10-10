# TODO / Handoff

## Status
- **Desktop FPS:** encode no longer blocks redraw. GL readback on main thread;
  FFmpeg/OpenH264 on a worker; `try_send` drops frames if worker is busy.
- **Green / deferred VideoInit:** SPS taken from FFmpeg **warm-up IDR** (seed was
  a P-frame without param sets). Published before the event loop.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# Desktop window should stay smooth (~display refresh).
# Log: FFmpeg … warm-up SPS/PPS …B  then live video SPS/PPS
#      VideoInit … param sets NN B (not deferred forever)

RELIVEVR_VIZ=1 RELIVEVR_ENCODER=nvenc cargo run
```

## Next
1. Confirm non-green scene on HMD.
2. nvenc for native 1440 @ 75.

## Bundle
`/home/workdir/artifacts/relivevr-linux-066.1-async-encode-warmup-sps-f993e2b.bundle`
