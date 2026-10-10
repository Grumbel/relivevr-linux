# TODO / Handoff

## Status
- **088.1:** Diagnostics: `hmd_sensor_time()` helper, log `hmd_poses=` on live
  frames. APK: QueryAndSendSensors AddHeadPose + DeviceEvent share one Pose.time.
- Code: 087.1 freshest HMD ptsSensor at send; 085.1 HMD-only.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
```
Server: `hmd_poses` should climb quickly; `ptsSensor` non-zero.
Client: `sensor pts not found` / `prev=0` should stop if HMD stamp sticks.

## Bundle
`/home/workdir/artifacts/relivevr-linux-088.1-hmd-pose-diagnostics-f993e2b.bundle`
