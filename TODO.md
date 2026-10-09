# TODO / Handoff

## Done
- Dual decoder slots (frmType 0 + 1) both fed.
- Solid colours confirmed (both blue; then left blue / right red).
- Complex per-eye test pattern: left = dark-blue + cyan grid + "LEFT";
  right = dark-magenta + pink grid + "RIGHT" (1440×1440 baseline IDR).

## Test
```bash
nix run .
# Expect: left eye grid labelled LEFT, right eye grid labelled RIGHT
```

## Next
1. Confirm complex pattern on headset (resolution / eye independence visible)
2. Pose / controller path (channel 7 DeviceEvent, SendSensorData / SendControllerData)
3. Clean up VideoInit spray (narrow type/channel that actually works)
4. Optional: animated sequence or proper P-frames instead of tiny placeholders
5. Longer-term: OpenXR / monado / ALVR / WiVRn integration

## Notes
- Session still times out / rediscovers without continuous frames (~10 s).
- Type 9 treated as force-IDR keepalive.
- Base commit for bundles: 6813f93 (initial skeleton).
- Video path is solid enough for static / keyed images; pose is the next functional gap.
