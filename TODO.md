# TODO / Handoff

## Status
- VideoInit after ctrl caps; type0 + JSON + **NUL** + SPS/PPS (Windows pcap #7).
- Still waiting on client `{"Message":"StartSensor"}`.

## Test
```bash
nix run .
# Look for StartSensor after VideoInit
```

## Next
1. Side-by-side pcap of our server vs Windows if still no StartSensor
2. Frida on StartSensor emission path
3. Pose parse once streaming

## Notes
- Base: 6813f93
