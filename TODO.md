# TODO / Handoff

## Done
- Video path solid: dual decoder, LEFT/RIGHT grid patterns confirmed.
- Channel demux + DeviceEvent logging.
- Live log (2026-10-09): connect → caps → StartRequest → video; **no channel-7
  pose packets**. Only type 0/7/5/3/9 on channel 0.
- Type 9 is 1-byte binary keepalive (not JSON); now triggers force-IDR again.
- PTS switched to wall-clock µs from stream origin (decoder lag was ~30s).

## Live observations (user log)
- HMD caps: `/hmd` DoF=true "AMD WVR Daydream"
- Ctrl caps: `/ctrlRight` Daydream, vol+/− click, haptic out, DoF=false
- StartRequest: 1440×1440 avc 60Hz SeparateEyeProcessing NLS IPD=0.064
- type=9 body=1 repeated ~every 200–400ms after stream starts
- No DEVICE_EVENT / channel 7 traffic while session was up
- logcat: Decoder lag ~31s excess — PTS/timing issue (addressed this tip)

## Test
```bash
nix run .
# Expect: LEFT/RIGHT grids, far less decoder-lag spam
# Move head + controller; note any new packet types/channels
```

## Next
1. Confirm decoder lag reduced with wall-clock PTS
2. Figure out why client sends no pose — possible causes:
   - needs ACK/reply to device caps
   - pose only after OpenVR driver “starts” tracking
   - different channel/type we still mis-classify
   - binary pose only when session fully “running” (maybe need audio path?)
3. Static RE of SendSensorData / SensorEngine in libwirelessvr-lib.so
4. Clean up VideoInit spray
5. Longer-term: monado / ALVR / WiVRn

## Notes
- Base commit for bundles: 6813f93.

## Bundle
`/home/workdir/artifacts/relivevr-linux-011.1-pts-type9-no-pose-yet-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-011.1-pts-type9-no-pose-yet-6813f93.bundle HEAD`
Tip: 064154f
