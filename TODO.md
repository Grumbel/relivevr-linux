# TODO / Handoff

## Status
- Hello loop fixed; headset connects cleanly.
- Still no client `StartSensor` after our VideoInit.
- Tip 024: single Windows-style VideoInit (flags=1, pure JSON, no spray).

## Test
```bash
nix run .
# Look for: StartSensor, then POSE DeviceEvent
```

## Next
1. If still no StartSensor: append codec NALs after VideoInit JSON
2. Try VideoInit after ctrl caps (pcap order)
3. Structured pose parse once streaming

## Notes
- Base: 6813f93
