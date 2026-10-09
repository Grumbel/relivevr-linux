# ReliveVR Wire Protocol

Status: partially reverse-engineered from `com.amd.wirelessgvr` 1.0.13
(`libwirelessvr-lib.so` arm64) + live traffic against a Lenovo Mirage Solo
class device reporting `DeviceType=VR-1541F`.

## Overview

| Direction        | Content                                      |
|------------------|----------------------------------------------|
| Client → Server  | Discovery HelloRequest (UDP broadcast :1235) |
| Server → Client  | HelloResponse (unicast or broadcast)         |
| Both             | Later: StartRequest, video/audio, pose, …    |

Default port: **UDP 1235** (TCP also supported by the client binary).

Client initiates discovery (`Motor::StartDiscovery` / `BroadcastMessage`).
Server only needs to reply (announcing helps some setups).

Manual client override (`app.settings` JSON on the headset):

```json
{"Connection":{"EnableDiscovery":false,"Server":"UDP://<host-ip>:1235"}}
```

Path is typically under `Android/data/com.amd.wirelessgvr/files/`.

---

## Layer 1 — FlowCtrl fragment header

Every datagram observed so far uses a **15-byte header**, multi-byte fields
**big-endian**, payload immediately after:

```c
struct FragmentHeader {
    uint16_t seq;     // +0  client discovery uses 0
    uint32_t field2;  // +2  == payload length for single-fragment messages
    uint32_t offset;  // +6  byte offset of this fragment in the full message
    uint32_t length;  // +10 this fragment's payload length
    uint8_t  flags;   // +14 0 observed so far
    // uint8_t payload[length];  // starts at +15
};
```

Constraints (from `Fragment::ParseFromBuffer`):

- Packet size must equal `length + 15`.
- Multi-fragment reassembly matches on seq / offset / length.
- `field2` on the wire for single-fragment discovery equals `length`
  (not total packet size). Earlier RE had guessed total size; live traffic
  corrected this.

Classes: `awvr::FlowCtrlProtocol::{Fragment,Buffer}`,
`FragmentMessage`, `ProcessFragment`, `StreamFlowCtrlProtocol::PrepareMessage`.

---

## Layer 2 — Control plane (type byte + JSON)

After fragment reassembly:

```
uint8_t type;     // 0 = Hello family (request and, currently, our response)
char    json[];   // text fed to AMF JSONParser
```

`Command::ParseBuffer` stores the type byte, then parses the remainder as JSON.

### Live HelloRequest (client → server)

Source example: `192.168.178.33` broadcasting to `255.255.255.255:1235`.

```
Fragment: seq=0 field2=159 offset=0 length=159 flags=0  (UDP payload 174 bytes)
type = 0
JSON:
{
  "DeviceID": "4b94589deb5e1561",
  "MaxDatagramSize": 65507,
  "Options": {
    "DeviceType": { "Type": "string", "Val": "VR-1541F" }
  },
  "ProtocolMinVersion": 1,
  "ProtocolVersion": 1
}
```

Notes:

- `ProtocolVersion` / `ProtocolMinVersion` = **1** (confirmed).
- `Options` uses AMF-variant-style objects: `{"Type":"string","Val":"…"}`.
- Client rediscovers about every 6s from a new ephemeral source port until it
  accepts a server (or crashes).

### HelloResponse (server → client)

Keys recovered from `HelloResponse::FromJSON` + rodata:

| Key                 | Type (observed / inferred)     | Notes                          |
|---------------------|--------------------------------|--------------------------------|
| ProtocolVersion     | int                            | 1                              |
| ProtocolMinVersion  | int                            | 1                              |
| MaxDatagramSize     | int                            | 65507 typical                  |
| DeviceID            | string                         | server id                      |
| Options             | object (AMF variant style)     | optional in our trials         |
| ServerName          | string                         | display name                   |
| ChannelsSupported   | array of ≤8 bools              | fills Channel support table    |
| Transports          | array of strings               | e.g. `["UDP"]`                 |

**Current status:** the client receives our unicast HelloResponse and then
the ReliveVR app dies or keeps rediscovering. The exact accepted response
shape is still being bisected via `RELIVEVR_STYLE` / `RELIVEVR_TYPE`
(see probe).

Response styles implemented in the probe:

- `minimal` — versions, MaxDatagramSize, DeviceID, ServerName only
- `echo` — client-like fields + ServerName
- `full` — all known keys including ChannelsSupported + Transports
- `RELIVEVR_TYPE=0|1` — type byte override

---

## Other JSON control messages (from symbols + FromJSON)

These are **not** yet seen on the wire; keys come from static RE.

### StartRequest (session parameters)

Constructor and FromJSON indicate:

- `DisplayModel` (string)
- `DisplayWidth`, `DisplayHeight` (int)
- `FrameRate` (float / rate)
- `Bitrate` related fields
- `InterpupillaryDistance`, `AspectRatio` (float)
- `SeparateEyeProcessing` (bool)
- `VideoCodec` (string)
- `NonLinearScalingSupported` (bool)

### VideoInit / VideoData (metadata only — not the bitstream)

**VideoInit:** `CodecID`, `NonLinearScaling`, resolution, bitrate, …

**VideoData** (per-frame timing / size):

- `ptsSensor`, `ptsServerLat`, `ptsEncoderLat`, `pts`
- `cmpFrmSize`, `frmType`, `encType`, `ptsSend`, `frameNum`

### AudioInit

- `SampleRate`, `Format`, `PTS`, …

### Other named messages

`StopRequest`, `UpdateRequest`, `VideoForceIDR`, `DeviceEvent` (pose),
`TrackableDeviceCaps`, `HelloRefused`, `StatLatency`, `VirtualWall`, …

---

## Channel

`awvr::Command::Channel` is a **small integer** (0–7 observed).

`ServerParametersImpl::IsChannelSupported(ch)` is simply:

```text
return table[base + 97 + ch];  // byte
```

`ChannelsSupported` in HelloResponse is a JSON array of bools written into
that table.

Exact role mapping (which channel carries binary video vs control vs pose)
is **unknown**.

---

## Binary video / audio path

Separate from JSON control:

- `Communicator` is constructed with `VideoReceiverCallback` and
  `AudioReceiverCallback`.
- Buffers go to `DisplayPipeline::SubmitSPSPPS` / `SubmitFrame` →
  `MediaCodecDecoder::SubmitInput` / `SubmitSPSPPS`.
- Client logs: `decoder start L eye pts=…`, `decoder start R eye pts=…`.
- MIME hints in binary: `video/` + `hevc`, `audio/mp4a-latm`.
- Separate left/right eye processing and non-linear scaling are supported.

**On-wire framing of the compressed bitstream (NAL units, length prefixes,
which Channel, etc.) is not yet known.**

---

## Discovery / session flow (best current model)

```
Client                         Server (our probe)
  |  UDP broadcast HelloRequest (type=0 JSON)     |
  |---------------------------------------------->|
  |  UDP unicast HelloResponse (type=0 JSON)      |
  |<----------------------------------------------|
  |  (client should stop rediscovering)           |
  |  … StartRequest / channel setup / video …     |
  |  (not yet observed)                           |
```

Open questions marked in `TODO.md`.

## HelloResponse required fields (crash-confirmed)

`HelloResponse::FromJSON` **null-dereferences** if these keys are absent:

- `MaxDatagramSize` (int)
- `DatagramSize` (int) — separate from MaxDatagramSize
- `Port` (int16 stored; use 1235)

Minimum viable response body (plus type byte 0):

```json
{
  "ProtocolVersion": 1,
  "ProtocolMinVersion": 1,
  "MaxDatagramSize": 65507,
  "DatagramSize": 65507,
  "Port": 1235,
  "DeviceID": "relivevr-linux-probe",
  "ServerName": "ReliveVR Linux Probe"
}
```


## Post-Hello type 7 (live, after crash fix)

After a successful HelloResponse (no SIGSEGV), the client immediately
sends another single-fragment packet:

```
type = 7
JSON ≈ same as HelloRequest
  {"DeviceID":"…","MaxDatagramSize":65507,
   "Options":{"DeviceType":{"Type":"string","Val":"VR-1541F"}},
   "ProtocolMinVersion":1,"ProtocolVersion":1}
```

Source port is a new ephemeral port (not the original broadcast port).
Likely a second SERVICE_OP_CODE / connect step. Probe replies with
HelloResponse using type 7 when the request was type 7.

Client may still re-broadcast type 0 every ~10s until the full handshake
completes — investigate whether type-7 response content/type is accepted.

## SERVICE_OP_CODE / CHANNEL_SERVICE (from client logcat)

```
ConnectToServerAndQueryParameters() send CHANNEL_SERVICE::SERVICE_OP_CODE_HELLO_DIRECT
OnMessageReceived() received CHANNEL_SERVICE::7
Failed to connect to discovered server UDP://192.168.178.48:1235
```

| Value | Name (logcat) | Role |
|------|----------------|------|
| 0 | (discovery HelloRequest) | Broadcast discovery |
| **7** | **SERVICE_OP_CODE_HELLO_DIRECT** | Directed connect after discovery |

Flow after discovery accepts our server:

1. Client connects to `UDP://<server-ip>:1235`
2. Sends type=7 HELLO_DIRECT (same JSON as discovery HelloRequest)
3. Expects a parseable HelloResponse → ServerParameters
4. On failure/timeout (~10s): `Failed to connect to discovered server …`

Discovery HelloResponse was enough to *select* the server; HELLO_DIRECT
response must supply fields needed to finish `QueryParameters` (likely
including ChannelsSupported, Transports, DatagramSize, Port, etc.).

## Connect-path message types (OnMessageReceived)

From `AWVRClientImpl::OnMessageReceived` disassembly:

| Payload type | Connect-path behaviour |
|--------------|------------------------|
| **0** | Parse as HelloResponse → build ServerParameters (**success**) |
| **1** | Treated as refusal / failure (sets error state) |
| **7** (or other) | Logged only (`CHANNEL_SERVICE::N`); **does not** complete QueryParameters |

Therefore a reply to `SERVICE_OP_CODE_HELLO_DIRECT` (request type 7) **must use type byte 0**.
Replying with type 7 explains the 10s timeout despite `OnMessageReceived() received CHANNEL_SERVICE::7`.
