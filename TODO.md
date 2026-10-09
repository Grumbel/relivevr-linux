# TODO / Handoff

## Done this tip
- Dual-eye: both decoder slots fed (frmType 0 + 1).
- User confirmed: both eyes solid blue.
- Per-eye colour verification: left = solid blue, right = solid red IDR.

## Test
```bash
nix run .
# Expect: left eye solid blue, right eye solid red
```

## Next
1. Confirm left blue / right red (proves independent decoder slots)
2. Optional: proper P-frames or continuous colour-keyed stream
3. Pose / controller path (channel 7 DeviceEvent, SendSensorData)
4. Clean up VideoInit spray (narrow type/channel that actually works)
5. Longer-term: OpenXR / monado / ALVR / WiVRn integration

## Notes
- Session still times out / rediscovers without continuous frames (~10 s).
- Type 9 treated as force-IDR keepalive.
- Base commit for bundles: 6813f93 (initial skeleton).
