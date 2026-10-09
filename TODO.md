# TODO / Handoff

## Done
- Image on headset (left blue with mono; SBS blue/red in tip)
- PTS + ~30 fps continuous stream
- Channel = fragment flags; video = channel 1

## Verify next run
```bash
nix run .
# Expected: left blue, right red (SBS)
# Watch decoder-full rate drop vs previous 60fps flood
```

## Backlog
1. Confirm SBS stereo; adjust if client expects different layout
2. Optional: load external .h264 via env path instead of embedded
3. Pose / DeviceEvent (channel 7)
4. Long-term: ALVR/WiVRn merge or OpenXR
