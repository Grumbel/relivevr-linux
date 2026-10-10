# Windows dumpsmall.pcapng — wire truth

Source: `dumpsmall.pcapng` (Windows ReliveVR server ↔ Daydream client).
Parsed 2026-10-10. Counts and values below are from direct extraction of
UDP payloads, not inference.

Goal: document **exactly** what Windows sends so the Linux server can match
it. Do not invent alternate PTS models without new evidence from this file
or a new capture.

---

## Session order (S→C / C→S)

Observed control sequence:

1. Discovery / Hello (client `ProtocolVersion` 1; server Hello uses
   `ProtocolVersion` **2**, `ServerName` like `DESKTOP-TCLQNM6`,
   `MaxDatagramSize` 65507).
2. Client device caps + StartRequest (`DisplayWidth/Height` 1440,
   `FrameRate` 60, `VideoCodec` path as in StartRequest JSON).
3. **VideoInit** (S→C) — see below.
4. **StartSensor** (S→C): `{"Message":"StartSensor"}`.
5. Client floods DeviceEvent poses (C→S) at high rate.
6. **VideoData** frames (S→C) on FlowCtrl channel 1.
7. Occasional C→S `{"FrameRate":…}` (values ~72.25, ~74.91 appear later;
   one outlier ~10893 early).
8. End: `{"Message":"StopSensor"}`.

---

## VideoInit (S→C)

JSON (exact shape from capture):

```json
{
  "BitDepth": 8,
  "CodecID": "hevc",
  "Height": 1440,
  "ID": 16893074446,
  "NonLinearScaling": false,
  "Viewport": [0, 0, 1440, 1440],
  "Width": 1440
}
```

Wire body after FlowCtrl (flags=1):

```text
[0x00][JSON][NUL][Annex-B HEVC param sets]
```

Param sets (HEVC NAL types):

| NAL | Type | Role |
|-----|------|------|
| first | 32 | VPS |
| second | 33 | SPS |
| third | 34 | PPS |

**CodecID is `hevc`.** This dump is not H.264/AVC.

`ID` is a large session-ish integer (not 0). `NonLinearScaling` is **false**
in Windows VideoInit even when the client advertises scaling support.

---

## StartSensor (S→C)

```json
{"Message":"StartSensor"}
```

FlowCtrl length field 0x1a (26) matches type byte + JSON without requiring a
NUL in the length accounting used on this path. After this, the client emits
pose DeviceEvents.

---

## VideoData (S→C) — 1248 JSON blobs

### Field order (fixed)

```text
cmpFrmSize, encType, frameNum, frmType,
pts, ptsEncoderLat, ptsSend, ptsSensor, ptsServerLat
```

Example (first left eye, frame 0):

```json
{
  "cmpFrmSize": 47708,
  "encType": 0,
  "frameNum": 0,
  "frmType": 0,
  "pts": 0,
  "ptsEncoderLat": 95723,
  "ptsSend": 0,
  "ptsSensor": 17915786108751744,
  "ptsServerLat": 453512
}
```

### Field meanings (from values, not guesses)

| Field | Observed behaviour |
|-------|--------------------|
| **frameNum** | 0…N; **same** for left and right of a stereo pair |
| **frmType** | `0` left, `1` right |
| **encType** | `0` on IDR (frameNums 0, 120, 240, 360, 480, 600), else `2` |
| **cmpFrmSize** | NAL payload size (min ~71, max ~123568, mean ~33k) |
| **pts** | Presentation timeline (see next section) |
| **ptsSensor** | **Exact** client pose sample `time` (~1e16). 624/624 left frames match a `"time"` present in DeviceEvents in this capture |
| **ptsSend** | Small latency **0…~6699** (median ~1456). **Not** equal to `pts` |
| **ptsEncoderLat** | ~56271…300703 (median ~140593) |
| **ptsServerLat** | ~186789…599130 (median ~285232) |

### Body framing

```text
FlowCtrl flags = 1 (video channel)
body = [0x01][VideoData JSON][NUL][Annex-B NAL units]
```

One FlowCtrl message per eye frame; large datagrams (IP-fragmented on the
wire). Not small FlowCtrl 1400B splits.

Stereo pair consistency: for every `frameNum`, both eyes share the same
`pts`, `ptsSensor`, and `frameNum` (0 mismatched pairs in 624 pairs).

---

## pts timeline (presentation clock)

Early segment (frameNum 0…~99):

| frameNum | pts |
|----------|-----|
| 0 | 0 |
| 1 | 166666 |
| 2 | 333333 |
| 3 | 500000 |
| … | ≈ frameNum × 166666.67 |

If the unit is **100 nanoseconds**, step 166666 → **16.666 ms → 60 Hz**
(plausible for VR). If the unit were whole microseconds, that would be 6 Hz
(implausible). Prefer the 100 ns / 60 Hz reading unless a better reference
appears.

**pts is not a single constant × frameNum for the whole capture:**

| Region | Typical Δpts | Implied rate (100 ns unit) |
|--------|--------------|----------------------------|
| early | 166666 / 166667 | ~60 Hz |
| mid (fn ~100–199) | 1000 | different mode |
| later | 138889 / 138888 | ~72 Hz |
| later | 135135 / 135136 | ~74 Hz |

These later rates line up with client `{"FrameRate":72.25…}` and
`{"FrameRate":74.90…}` messages in the same file.

There is **no** stable linear map from `pts` to pose `time` (ratios and
two-point fits drift). Treat `pts` and `ptsSensor` as independent fields.

---

## ptsSensor and poses

**ptsSensor is always an `/hmd/pose` time**, never `/ctrlRight/pose` (624/624
left-eye frames in dumpsmall). DeviceEvents carry both HMD and controller in one
message (HMD first, controller ~1 µs later). The server must not stamp controller
times into VideoData.

## ptsSensor and poses (detail)

DeviceEvent pose sample (C→S), abbreviated:

```json
{
  "events": [{
    "id": "/hmd/pose",
    "data": [{
      "time": 17915786107968154,
      "val": {
        "baseFrmIdx": 17,
        "frmIdx": 17,
        "orient": [qx, qy, qz, qw],
        "pos": [x, y, z],
        ...
      }
    }]
  }, ...]
}
```

Facts:

- Pose `"time"` values are large (~1.79×10¹⁶), unique per sample in this file
  (11260 large `time` fields).
- **Every** video `ptsSensor` equals some pose `time` exactly.
- ~85 pose samples occur **before** the first video frame’s `ptsSensor`.
- Pose **`frmIdx` does not equal** video `frameNum` (e.g. video fn=0 →
  pose frmIdx=22). Do not assume Present indexes by frmIdx == frameNum.

---

## Logcat strings vs this dump

Client warnings seen on Linux tests:

- `Pose for Present pts=0 not found`
- `input frame sensor pts not found = <time> prev=0`

**Windows frame 0 also has `pts=0`.** So “Present pts=0” is not by itself
proof that the Linux server chose the wrong presentation tick; it is the
same first presentation index Windows uses.

`sensor pts not found` with `prev=0` means the client never successfully
resolved **any** `ptsSensor` time into its local sensor/pose table. The dump
requires `ptsSensor` to be a real pose `time`. If that lookup always fails,
Present never unblocks and the decode queue grows — independent of whether
`pts` is `frameNum*166666` or another schedule.

Possible causes (evidence-limited; need a Linux-side capture or APK study):

- Pose sample already aged out of a short client queue under high lag
  (Linux often ~80–300 ms between frames; Windows still reports large
  `ptsServerLat` but sustains much higher fps).
- Time base / queue keying differs from the DeviceEvent `time` we echo
  (not visible in this pcap alone).

---

## What Linux must match (checklist)

From this dump only:

1. VideoInit: `CodecID=hevc` (for bit-exact match), 1440², VPS+SPS+PPS trailer,
   `NonLinearScaling` as Windows (false here), non-zero `ID`.
2. StartSensor after VideoInit; wait for client poses before expecting
   non-zero `ptsSensor` (Windows already has poses by frame 0).
3. VideoData JSON field order and roles above.
4. `pts` ≈ 60 Hz timeline early (`×166666` in the observed unit); do not
   assume one constant for the whole session.
5. `ptsSensor` = exact pose `time` from DeviceEvents (never a second copy of
   `pts`).
6. `ptsSend` = small latency, **never** set equal to `pts`.
7. `encType` 0 on IDR, 2 on P; IDR every 120 frames in this capture.
8. Same `frameNum`/`pts`/`ptsSensor` for both eyes; `frmType` 0/1.
9. Body `[1][JSON][NUL][Annex-B]`, large single FlowCtrl datagram per eye.
10. No one-shot connect frames with `ptsSensor=0` or experimental ticks
    (e.g. 16666) — Windows does not do that.

AVC (openh264 / libx264) is a separate experimental path; **this file is HEVC**.

---

## Extraction notes

- File is pcapng; full EPB/UDP parse was not required — VideoData and control
  JSON were located by payload regex on the raw file.
- 1248 VideoData JSONs (624 stereo pairs if counting left+right).
- Pose `time` and video `ptsSensor` equality verified by set membership over
  the whole capture.

When behaviour diverges on Linux, prefer a **new capture of the Linux
session** next to logcat over changing `pts` multipliers without evidence.

Client-side lookup (APK): [client-apk.md](client-apk.md).


## Linux implementation notes (084.1)

The server is the sender: it must emit the same field roles as this dump.

**Head tracking ↔ image:** Windows sets `ptsSensor` to the pose sample `time`
associated with that frame. On Linux the OpenGL path stamps `LatestPoses.latest_time`
at **render/readback** into `LiveVideo.pose_time`, and the UDP path copies that into
`ptsSensor`. Frames are not sent until a non-zero pose stamp exists (after StartSensor
poses), matching Windows frame 0 already carrying a real `ptsSensor`.

`pts` remains the independent presentation clock (`frameNum * 166666` for the early
60 Hz segment in 100 ns units).
