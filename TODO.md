# TODO / Handoff

## Done
- Video path solid: dual decoder, left/right grid+label patterns confirmed.
- Channel-aware demux: fragment flags = channel; StreamFlowCtrl peel;
  channel 7 DeviceEvent / binary pose logging with f32 LE/BE preview.
- Device caps (type 5) and type-4 DeviceEvent JSON path logged.

## Test (pose RE)
```bash
nix run .
# Wear headset, move head + Daydream controller.
# Watch log for:
#   DEVICE_EVENT ch=7 type=… hex=… floats=LE[…] BE[…]
#   binary type=… ch=… floats=…
# Paste interesting packets into docs/re-notes.md / open an issue for struct layout.
```

## Next
1. Capture live pose packets (channel 7 and any binary service types) while moving head/controller
2. Map quaternion + position + button bits from float/hex dumps
3. Expose latest HMD + controller state (stdout or shared memory) for OpenXR/SteamVR later
4. Clean up VideoInit spray
5. Optional: animated test pattern / proper P-frames
6. Longer-term: monado / ALVR / WiVRn integration

## Notes
- Pose wire format is still RE-open; probe now demuxes and dumps everything.
- Type 9 still treated as force-IDR keepalive.
- Base commit for bundles: 6813f93.
