# TODO / Handoff

## Milestone
- **VideoInit accepted:** `ReInit(avc)` YES, 1440×1440 decoder ready.
- Session dies ~10s without frames (`StartDiscovery: Reset decoder`).

## Next (priority order)
1. **Binary H.264** — SPS/PPS via SubmitSPSPPS path, then IDR frames
   - RE: channel ID, length prefix, relationship to VideoData JSON
   - Minimal: static grey/black IDR loop at 1440×1440
2. Narrow VideoInit type/channel (optional; spray works)
3. Keepalive / pose (DeviceEvent type 5 inputs)

## Test after sending frames
```bash
adb logcat -s AMF_TRACE:D | grep -iE 'Submit|SPS|Frame|Decoder|Error|Reset'
```
