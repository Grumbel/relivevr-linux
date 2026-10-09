# TODO / Handoff

## Milestone
- VideoInit → ReInit(avc) YES
- Channel = fragment **flags** (1 = video)
- Video frame format: `[1][VideoData JSON\0][Annex-B NALs]`

## Test
```bash
nix run .
adb logcat -s AMF_TRACE:D | grep -iE 'Frame|Submit|SPS|Decoder|Error|Reset|OnFrame'
```

## Next
1. Confirm OnFrameReceived from logcat
2. Real 1440×1440 SPS/PPS + IDR (ffmpeg) so picture displays
3. Continuous 60 fps stream + keepalive
