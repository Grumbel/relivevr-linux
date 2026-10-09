# TODO / Handoff

## Status
- **Pcap decoded**: pose is type-4 DeviceEvent after client `{"Message":"StartSensor"}`.
- StartSensor is **client→server**, after Windows-style VideoInit.
- Server Hello/VideoInit updated to match pcap (ChannelsSupported[4], hevc, VideoInit shape).

## Test
```bash
nix run .
# Expect after connect: StartSensor log, then POSE DeviceEvent lines
```

## Next
1. Confirm StartSensor + pose with updated Hello/VideoInit
2. Parse pose into structured log / OpenXR later
3. Optional: append codec NALs after VideoInit JSON (Windows does)
4. Clean VideoInit type spray

## Notes
- Base: 6813f93
- Capture: attachments/dump.pcapng (15 packets)
