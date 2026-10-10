# TODO / Handoff

## Status (from dumpsmall.pcapng + logcat)
- VideoInit/SPS accepted (`SubmitSPSPPS result=0`).
- Frames dropped: **FlowCtrl "Message is old"** for low seq IDs.
- Windows uses **HEVC**; we still send AVC (client asked avc — SPS accepted).
- Fixed: monotonic seq after VideoInit; encType 0/2; JSON field set; frag seq advance.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=x264 \
  RELIVEVR_ENCODE_W=720 RELIVEVR_ENCODE_H=720 cargo run
# logcat should NOT show "Message is old" for every early frame
```

## Next
- If still green: try HEVC encode path (Windows native).
- Pose/PTS queue warnings secondary.

## Bundle
(to be produced)
