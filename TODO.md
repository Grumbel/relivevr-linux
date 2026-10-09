# TODO / Handoff

## Current tip
- StartRequest (type 3) handled → sends VideoInit (Width/Height/CodecID/NonLinearScaling).
- VideoInit **type byte default 2** (unconfirmed). Bundle 016.1.

## Test
```bash
RELIVEVR_STYLE=full nix run .
# or try type bytes:
RELIVEVR_VIDEOINIT_TYPE=2 nix run .
RELIVEVR_VIDEOINIT_TYPE=4 nix run .
RELIVEVR_VIDEOINIT_TYPE=6 nix run .
```

Watch logcat:
```
adb logcat -s AMF_TRACE:D | grep -iE 'Video|Codec|Decoder|Start|CHANNEL|Error'
```

Success: no more `video/` empty MIME; decoder starts. Then need binary NALs.

## Next
1. Confirm VideoInit type byte (logcat / stop of video/ error)
2. Binary H.264 SPS/PPS + IDR framing + channel
3. Session keepalive (client rediscovers ~10s)
