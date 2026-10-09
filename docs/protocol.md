# ReliveVR Wire Protocol

Status: discovery + HELLO_DIRECT connect working against VR-1541F
(Lenovo Mirage Solo / Daydream). Next: acknowledge StartRequest and stream video.

## Overview

| Direction | Content |
|-----------|---------|
| Client → Server | Discovery HelloRequest (broadcast :1235) |
| Server → Client | HelloResponse type **0** |
| Client → Server | HELLO_DIRECT type **7** (unicast) |
| Server → Client | HelloResponse type **0** again |
| Client → Server | Device caps type **5**, StartRequest type **3** |
| Server → Client | (TODO) VideoInit / video bitstream |

Port: **UDP 1235**. TCP supported by client binary but live path is UDP.

---

## Layer 1 — FlowCtrl fragment header (15 bytes, BE)

```c
struct FragmentHeader {
    uint16_t seq;     // +0
    uint32_t field2;  // +2  == payload length (single-fragment)
    uint32_t offset;  // +6
    uint32_t length;  // +10
    uint8_t  flags;   // +14
    // payload[length] at +15
};
```

Packet size == length + 15.

---

## Layer 2 — type byte + JSON

### SERVICE_OP_CODE (type byte) — live + logcat

| Type | Name | Role |
|------|------|------|
| **0** | SERVICE_OP_CODE_HELLO | HelloResponse on connect path; also discovery HelloRequest from client |
| **1** | (refused) | Connect-path refusal |
| **3** | (StartRequest) | Session/display/video params from client |
| **5** | (device / TrackableDeviceCaps) | HMD + controller capability JSON |
| **7** | SERVICE_OP_CODE_HELLO_DIRECT | Directed connect after discovery |
| **9** | (keepalive) | Single-byte `09`. Bursts when video starves; ~0 when stream is healthy. **Do not** reply with IDR. |

Connect path (`AWVRClientImpl::OnMessageReceived`): only **type 0** completes
QueryParameters; type 1 = fail; other types only logged.

### HelloRequest (client, type 0 or 7)

```json
{
  "DeviceID": "4b94589deb5e1561",
  "MaxDatagramSize": 65507,
  "Options": {"DeviceType": {"Type": "string", "Val": "VR-1541F"}},
  "ProtocolMinVersion": 1,
  "ProtocolVersion": 1
}
```

### HelloResponse (server, **must be type 0**)

Required (null-deref if missing): `MaxDatagramSize`, `DatagramSize`, `Port`.

Working Full example:

```json
{
  "ProtocolVersion": 1,
  "ProtocolMinVersion": 1,
  "MaxDatagramSize": 65507,
  "DatagramSize": 65507,
  "Port": 1235,
  "DeviceID": "relivevr-linux-probe",
  "Options": {"DeviceType": {"Type": "string", "Val": "PC"}},
  "ServerName": "ReliveVR Linux Probe",
  "ChannelsSupported": [true,true,true,true,true,true,true,true],
  "Transports": ["UDP"]
}
```

### StartRequest (client → server, type 3) — live

```json
{
  "AspectRatio": 1.0,
  "Bitrate": 50000000,
  "DisplayHeight": 1440,
  "DisplayModel": "AMD WVR Daydream",
  "DisplayWidth": 1440,
  "FrameRate": 60.0,
  "InterpupillaryDistance": 0.0640000030398369,
  "NonLinearScalingSupported": true,
  "SeparateEyeProcessing": true,
  "VideoCodec": "avc"
}
```

Stereo total **1440×1440**, 60 Hz, **H.264 (avc)**, 50 Mbit/s, separate-eye + NLS.

### Device caps (client → server, type 5) — live

HMD:
```json
{"DoF":true,"class":"hmd","desc":"AMD WVR Daydream","id":"/hmd","virtWall":false}
```

Controller:
```json
{
  "DoF": false,
  "class": "ctrl",
  "desc": "Daydream",
  "id": "/ctrlRight",
  "inputs": ["/ctrlRight/in/vol/+/click","/ctrlRight/in/vol/-/click","/ctrlRight","/ctrlRight"],
  "outputs": ["/ctrlRight/out/haptic"],
  "virtWall": false
}
```

### Successful connect logcat

```
send CHANNEL_SERVICE::SERVICE_OP_CODE_HELLO_DIRECT
OnMessageReceived() received CHANNEL_SERVICE::SERVICE_OP_CODE_HELLO
ConnectToServerAndQueryParameters() - succeeded
Connected to UDP://192.168.178.48:1235
HMD Connect() request sent
```

Then MediaCodec tries `video/` (empty codec MIME) until server sends VideoInit:
`AMediaCodec_createDecoderByType(video/) failed`.

---

## Binary video (next)

- Client wants **avc** (H.264).
- MIME must become `video/avc` via VideoInit (JSON keys: CodecID, etc.).
- Bitstream path: VideoReceiverCallback → SubmitSPSPPS / SubmitFrame → MediaCodec.
- Channel ID for binary video still unknown.

## Channel

Small int 0–7; `ChannelsSupported` bool[8] fills support table.

## StartRequest (type 3) — handled

Client → server after connect. Server should follow with VideoInit.

## VideoInit (server → client)

`VideoInit::FromJSON` keys:

| Key | Type | Notes |
|-----|------|-------|
| Width | int | e.g. 1440 |
| Height | int | e.g. 1440 |
| CodecID | string | `"avc"` → client builds MIME `video/avc` |
| NonLinearScaling | bool | match StartRequest |

**Type byte not yet confirmed on wire.** Probe defaults to **2**; override with
`RELIVEVR_VIDEOINIT_TYPE`. If MediaCodec still fails on `video/`, try other types
(4, 6, 8, …) and watch AMF_TRACE.

Example:
```json
{"Width":1440,"Height":1440,"CodecID":"avc","NonLinearScaling":true}
```

## Additional opcodes (constructors)

| Type | Class |
|------|-------|
| 6 | UpdateRequest |

VideoInit type still unknown; probe brute-forces 2,4,8,9,10 with CodecID
variants `avc` / `video/avc`.

## StreamFlowCtrl message header (session path)

`StreamFlowCtrlProtocol::PrepareMessage` wraps the body in 7 bytes before
fragmentation:

```
u32 BE  body_length
u8      channel          // Command::Channel 0..7
u16 BE  stream_seq
u8[]    body             // type byte + JSON (or binary)
```

Discovery / HELLO on the datagram discovery session do **not** use this header
(live Hello is bare type+JSON in the fragment). Post-connect video/control may
require it — probe sends both forms for VideoInit.

## VideoInit status (live confirmed 2026-10-09)

After StartRequest + VideoInit spray, client logcat:

```
MediaCodecDecoder Error: createDecoderByType(video/) failed   // benign at connect
Motor: ReintDecoder: NO
MediaCodecDecoder Info: ReInit(avc) … width:1440 height:1440 color-format…
Motor: ReintDecoder: YES
```

Decoder is configured for **avc @ 1440×1440**. Initial empty-MIME failure is
expected; ReInit after VideoInit succeeds.

~10s later: `StartDiscovery: Reset decoder` — session drops without video frames.

**Next:** binary H.264 (SPS/PPS + IDR) on the video channel so the session stays up.

## Channel = fragment flags byte

`ProcessFragment` stores fragment header **flags** (offset 14) as the Buffer
channel. Valid channels in `Communicator::OnMessageReceived`:

| Channel | Role |
|---------|------|
| 0 | SERVICE (Hello, StartRequest, …) |
| 1 | VIDEO → `Motor::OnFrameReceived` |
| 2 | AUDIO → `Motor::OnAudioReceived` |
| 7 | DeviceEvent (type 4 only) |

## Video frame payload (`OnFrameReceived`)

```
u8   msg_kind     // 1 = VideoData JSON path
char json[]       // NUL-terminated VideoData JSON
u8   nals[]       // H.264 Annex-B (remainder of buffer)
```

VideoData JSON keys: ptsSensor, ptsServerLat, ptsEncoderLat, pts, cmpFrmSize,
frmType, encType, ptsSend, frameNum.

StreamFlowCtrl 7-byte header is for the TCP/stream path; UDP datagrams use
flags as channel directly.

## H.264 payload (probe)

Probe embeds a real libx264 baseline 1440×1440 blue IDR (SPS+PPS+SEI+IDR)
and sends it on channel 1 after StartRequest, followed by 30 P-frames.

## Live video path confirmed

Client logcat after channel-1 frames:
```
DisplayPipeline: Decoder lag around frame #3
```
Frames reach MediaCodec. Session still resets ~10s without continuous stream.

## Type 9 (client → server)

Single-byte payload `09` after frames start. Treated as force-IDR / keepalive;
probe responds with IDR and keeps the stream target.

## Image on headset (2026-10-09)

First successful video: solid blue visible in left eye. Decoder reported
input full / lag when flooding at 60 fps with PTS=0.

Fixes in tip:
- PTS / ptsSensor / ptsSend = frameNum * 16666 µs
- Stream ~30 fps; smaller initial burst
- SBS test pattern 1440×1440 (left blue, right red) for stereo check

`SeparateEyeProcessing: true` in StartRequest — stereo layout TBD
(SBS vs dual stream).

## Stereo layout (working hypothesis)

StartRequest advertises `SeparateEyeProcessing: true` and 1440×1440.
`RenderEngine::RenderEye` takes per-eye viewport; GVR buffer viewports sample
the decoded texture.

Live result with full-frame blue (frmType 0 only): **left eye blue, right dark**.

With frmType 0 **and** 1 (same blue IDR): **both eyes solid blue** (user confirmed).

Independent solid colours (user confirmed):
- frmType 0 → solid blue IDR (left)
- frmType 1 → solid red IDR (right)

Current probe patterns (complex verification):
- frmType 0 → dark-blue + cyan 90px grid + centered "LEFT"
- frmType 1 → dark-magenta + pink 90px grid + centered "RIGHT"

`isSeparateEyeProcessing` JNI reads a Settings bool at offset +120.

## Pose / controllers (in progress)

- Client sends device caps type 5 (`/hmd`, `/ctrlRight`) after connect.
- Channel **7** = DeviceEvent path (`Communicator` jump table).
- Type **4** on service/channel-7 = DeviceEvent (JSON or binary — live TBD).
- `Communicator::SendSensorData` / `SendControllerData` — client→server.
- Daydream controller inputs listed in caps JSON
  (`/ctrlRight/in/vol/+/click`, trackpad, app button, haptic out).

Probe (tip 010+):
- Demuxes by fragment `flags` (= channel).
- Peels optional StreamFlowCtrl 7-byte header.
- Logs channel-7 and other binary bodies as hex + LE/BE f32 preview
  so live captures can pin quaternion/position layout.

Wire struct for continuous pose is still open — needs live packet dump while
moving the headset / Daydream controller.

## SeparateEyeProcessing / frmType (confirmed)

With `SeparateEyeProcessing: true`, `Motor::OnFrameReceived` selects one of two
decoder slots via `frmType`:

```
slot = (frmType | 2) != 2  →  frmType 0 → left, frmType 1 → right
```

Both eyes need SPS/PPS + IDR. Sending only frmType=0 left the right eye black
while the left showed the full frame (including SBS split down the middle).

Probe sends every access unit twice: frmType 0 and 1.
