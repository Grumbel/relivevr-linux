# TODO / Handoff

## Status
- **Root cause of FFmpeg AU timeout found:** `-fflags +nobuffer+flush_packets`
  makes rawvideo→libx264 emit *zero* frames on current FFmpeg. Removed.
- Oneshot fallback still present as safety net (not the primary path).
- Viz pacing: Poll + request_redraw + min_dt; not artificially low.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 cargo run
# expect quickly:
#   FFmpeg libx264 … (warm-up OK, AU NNNB try 0)
#   live video SPS/PPS …B

RELIVEVR_VIZ=1 cargo run   # auto nvenc/…
```

## Next
1. Confirm live scene on HMD at usable fps.
2. force_idr / distortion / OpenXR.

## Bundle
`/home/workdir/artifacts/relivevr-linux-063.1-fix-fflags-drops-frames-f993e2b.bundle`
