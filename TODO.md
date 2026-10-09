# TODO / Handoff

## Status
- CStartSensor elicits client **type=4 JSON `{}`** reply (first DeviceEvent!).
- No continuous pose yet; session rediscovered ~5s after probe.
- Tip 017: delayed minimal variants on service type 4 only.

## Test
```bash
RELIVEVR_START_SENSOR=1 nix run .
# ~2s after video starts, three CStartSensor variants
# Watch for type=4 with non-empty JSON or orient/pos
```

## Next
1. Capture any non-empty type-4 after variants
2. Disasm DeviceEvent::FromJSON for required fields
3. If empty {} only: SensorThread::SetActive not reached — find callers

## Notes
- Base: 6813f93

## Bundle
`/home/workdir/artifacts/relivevr-linux-017.1-cstartsensor-variants-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-017.1-cstartsensor-variants-6813f93.bundle HEAD`
Tip: 0192355
