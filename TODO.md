# TODO / Handoff

## Status
- Desktop encode is async (smooth window).
- **Green HMD:** likely oversize single-datagram video frames (IP fragment loss)
  and/or missing in-band SPS. Now:
  - Multi-fragment FlowCtrl packets (1400 B payload chunks)
  - `repeat-headers=1` on x264
  - Prepend stored SPS/PPS to IDRs that lack them

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# expect: live frame #N L=…B R=…B
# VideoInit with non-zero param sets; scene on HMD (not green)
```

## Bundle
(to be produced)
