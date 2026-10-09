# TODO / Handoff

## Status
- VideoInit deferred until **ctrl** TrackableDeviceCaps (Windows pcap order).
- Hello loop fixed earlier.

## Test
```bash
nix run .
# Order should be: Hello → hmd caps → StartRequest → ctrl caps → VideoInit → …
# Look for StartSensor + POSE after VideoInit
```

## Next
1. Confirm StartSensor
2. Codec NALs after VideoInit if still missing
3. Pose parse

## Notes
- Base: 6813f93
