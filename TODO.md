# TODO / Handoff

## Status
- **084.1:** Match Windows image↔tracking binding:
  - At **render** time, stamp client pose `time` onto the stereo AU (`LiveVideo.pose_time`).
  - VideoData `ptsSensor` = that stamp (not “whatever pose is latest at UDP send”).
  - Do not send video until `pose_time != 0` (Windows frame 0 already has a real
    sensor time after StartSensor poses).
  - `pts = frameNum * 166666`; `ptsSend` small; `ptsServerLat` ≈ publish→send µs.
  - VideoInit `ID` = unix secs (non-zero large-ish).
- See `docs/windows-pcap.md`. Prefer `RELIVEVR_ENCODER=hevc` for codec match.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# closer to dump:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc cargo run
```
Server log should show `ptsSensor=` non-zero on first live frame. No frames before poses.

## Bundle
`/home/workdir/artifacts/relivevr-linux-084.1-pose-bind-at-render-f993e2b.bundle`
