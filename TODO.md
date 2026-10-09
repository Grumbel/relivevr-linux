# TODO / Handoff

## Status
- **cap2 breakthrough**: StartSensor is **server → client** (type 5), not client→server.
- After VideoInit we now send `{"Message":"StartSensor"}`; expect pose uplink.
- Video still armed for display.

## Test
```bash
nix run .
# Expect: -> StartSensor … type=5 (S→C)
# Then: *** POSE DeviceEvent with orient/pos
```

## Notes
- Base: 6813f93
- Captures: dump.pcapng, cap2.pcapng

## Bundle
`/home/workdir/artifacts/relivevr-linux-030.1-startsensor-s2c-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-030.1-startsensor-s2c-6813f93.bundle HEAD`
Tip: 9a4a423
