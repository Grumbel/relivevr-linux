# Client APK RE — Present / sensor lookup

APK: `com.amd.wirelessgvr` 1.0.13 (WirelessGVR / Daydream client).  
Native library: `libwirelessvr-lib.so` (arm64-v8a).

## Call chain

```
OnFrameReceived(type, body, len)
  body[0] == 1  → VideoData path
  Parse JSON (pts, ptsSensor, frameNum, frmType, …)
  Submit NALs to MediaCodecDecoder
  Insert map entry:  present_time (pts)  →  sensor_time (ptsSensor)

Motor::GetHeadPoseByPresentTime(present_time, &sensor_out, &pose_out)
  1) GetSensorTimeFromPresentTime(present_time)
       exact match in list @ Motor+0x670
       key   = node+16  (pts)
       value = node+24  (ptsSensor)
       on hit: remove node (one-shot), return sensor_time
       on miss: return -1
  2) Walk head-pose queue @ Motor+0x648
       compare pose.time (node+120) to sensor_time  — exact match
       on hit: copy SensorEngine::Pose into pose_out
       on miss: AMFTraceW (Present / sensor not-found path)

Motor::AddHeadPose(Pose&)
  Inserts local HMD sample into the same pose queue, ordered by Pose.time
  (field at Pose+104 → stored at node+120).
```

## Implications for the server

1. **`pts` is the Present lookup key**  
   Client maps `pts → ptsSensor` when the frame arrives. Present later
   asks for pose by that same `pts`. Windows early segment uses
   `pts = 0, 166666, 333333, …` (frameNum × 166666).

2. **`ptsSensor` must be a time that exists in the local head-pose queue**  
   That queue is filled by the client’s own sensors (`AddHeadPose`), with
   the same `time` values it puts in DeviceEvent `/hmd/pose` samples.
   dumpsmall: **624/624** VideoData `ptsSensor` values are exact
   `/hmd/pose` times — **never** `/ctrlRight/pose`.

3. **Exact match, not nearest**  
   Both maps use equality. A controller time, or a slightly altered HMD
   time, will miss → `prev=0` / Present not found.

4. **One-shot present→sensor entries**  
   Successful Present lookup consumes the map node. Re-present of the same
   `pts` without a new frame will miss.

## Server requirements (aligned with dumpsmall + this RE)

| Field | Requirement |
|-------|-------------|
| `pts` | Presentation clock; Windows early = `frameNum * 166666` |
| `ptsSensor` | Exact **`/hmd/pose`** sample `time` for that image |
| `ptsSend` | Small latency (not equal to `pts`) |
| Stereo | Same `pts` / `ptsSensor` / `frameNum` for both eyes |

Linux tip **085.1**: only `/hmd/pose` updates `latest_time` used for
`ptsSensor` (controller times no longer overwrite).

## Symbols (arm64)

| Symbol | VA |
|--------|-----|
| `Motor::OnFrameReceived` | `0xdc388` |
| `Motor::AddHeadPose` | `0xdbb04` |
| `Motor::GetSensorTimeFromPresentTime` | `0xe028c` |
| `Motor::GetHeadPoseByPresentTime` | `0xe037c` |
| `Motor::UpdatePresentTime` | (export) |

JSON key strings in `.rodata`: `ptsSensor`, `ptsServerLat`, `ptsEncoderLat`,
`cmpFrmSize`, `frmType`, `encType`, `ptsSend`, `frameNum`.
