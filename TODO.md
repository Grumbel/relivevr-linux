# TODO / Handoff

## Done
- Dual decoder slots (frmType 0 + 1) both fed.
- User confirmed: both eyes solid blue.
- User confirmed: left blue / right red — independent decoder slots verified.

## Test (current tip)
```bash
nix run .
# Expect: left eye solid blue, right eye solid red
```

## Next
1. Pose / controller path (channel 7 DeviceEvent, SendSensorData / SendControllerData)
2. Clean up VideoInit spray (narrow type/channel that actually works)
3. Optional: proper P-frames or continuous colour-keyed / moving test pattern
4. Longer-term: OpenXR / monado / ALVR / WiVRn integration

## Notes
- Session still times out / rediscovers without continuous frames (~10 s).
- Type 9 treated as force-IDR keepalive.
- Base commit for bundles: 6813f93 (initial skeleton).
- Video path is solid enough for “send an image”; pose is the next functional gap.
