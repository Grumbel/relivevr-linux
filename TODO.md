# TODO / Handoff

## Status
- Video solid.
- Static RE of Oculus 1.0.26 `libwirelessvr-lib.so` + 2.0 `libawvr.so`:
  pose path, DeviceEvent JSON keys, `CStartSensor` / `StopSensor` found.
- Opt-in: `RELIVEVR_START_SENSOR=1` after StartRequest (may crash — off by default).

## Test
```bash
nix run .                                          # stable video only
RELIVEVR_START_SENSOR=1 nix run .                  # try enable sensors
# Watch for DEVICE_EVENT / JSON with orient/pos after the second form
```

## Next
1. Test CStartSensor opt-in; capture any new client→server packets
2. Refine DeviceEvent JSON from live dump (`orient`/`pos`/…)
3. If still no pose: deeper disasm of `SensorThread::Run` / `SetActive` callers
4. Clean VideoInit spray

## Notes
- Base: 6813f93
- APKs analyzed under /tmp (not in repo): GPUOpen 1.0.26 Oculus + 2.0 beta

## Bundle
`/home/workdir/artifacts/relivevr-linux-016.1-re-cstartsensor-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-016.1-re-cstartsensor-6813f93.bundle HEAD`
Tip: fe5db16
