# TODO / Handoff

## Current tip
- Real 1440×1440 baseline H.264 IDR embedded; sent on channel 1 after StartRequest.
- Format: flags=1, body=`[1][VideoData JSON\0][Annex-B NALs]`

## Test
```bash
nix run .
adb logcat -s AMF_TRACE:D | grep -iE 'Frame|Submit|SPS|Decoder|Error|Reset|color|Queue'
```

Look for SubmitSPSPPS / SubmitFrame / decode success vs errors.

## Next
1. Continuous 60 fps loop (timer) so session survives >10s
2. Fix NAL format if client expects AVCC length-prefix instead of Annex-B
3. Pose / controller path
