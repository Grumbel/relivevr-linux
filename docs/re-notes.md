# Reverse-Engineering Notes

Source binary: `com.amd.wirelessgvr` 1.0.13  
Native library: `lib/arm64-v8a/libwirelessvr-lib.so` (~1.7 MiB)  
Build: Android NDK clang 8.0.7, AMF 1.3-dev paths in strings.  
Namespaces: `awvr`, `amf`, `Comm`, `SensorEngine`, …

Entry point: `AWVRCreateClient`.  
Java side is thin JNI (`com.amd.wirelessgvr.WirelessHMDClient`).

---

## How analysis was done

1. `strings` / `nm -D` + `c++filt` for architecture map.
2. `aarch64-linux-gnu-objdump -d` on:
   - `FlowCtrlProtocol::Fragment::ParseFromBuffer` (0x9d588)
   - `Fragment` constructor / `FragmentMessage`
   - `Command::ParseBuffer` (0xa21e4)
   - `HelloResponse::FromJSON` / `HelloRequest::FromJSON`
   - `ServerDiscoverySession::OnCompleteMessage`
   - `VideoInit::FromJSON`, `VideoData::FromJSON`, `StartRequest::FromJSON`
   - `ServerParametersImpl::IsChannelSupported`
3. Rodata string recovery for JSON keys (stack-built strings via adrp/add).
4. Live UDP capture against VR-1541F (Lenovo Mirage Solo class).

APK path (session attachments):  
`/home/workdir/attachments/com.amd.wirelessgvr_1.0.13-13_minAPI24(arm64-v8a,armeabi-v7a)(nodpi)_apkmirror.com.apk`

---

## FlowCtrl fragment header (confirmed)

See `docs/protocol.md`. Summary:

- 15-byte header, BE multi-byte fields.
- Live single-fragment discovery: `field2 == length`.
- ParseFromBuffer requires `size >= 16` and `size == length + 15`.

---

## Control plane = type + JSON (confirmed)

`Command::ParseBuffer`:

1. Require size ≥ 2.
2. Store first byte as type on the Command object.
3. Assign remaining bytes to a string; feed to AMF `CreateJSONParser`.

Discovery type byte:

- `0` → client path parses as server Hello and builds `ServerParametersImpl`
  from `HelloResponse` + sender address.
- `1` → alternate branch in `OnCompleteMessage` (not fully RE'd).
- Other → ignored / early out.

---

## Live HelloRequest (2026-10-09)

```
from 192.168.178.33:ephemeral
seq=0 field2=159 off=0 len=159 flags=0  pkt=174

{"DeviceID":"4b94589deb5e1561",
 "MaxDatagramSize":65507,
 "Options":{"DeviceType":{"Type":"string","Val":"VR-1541F"}},
 "ProtocolMinVersion":1,
 "ProtocolVersion":1}
```

Device is Daydream-era Lenovo hardware (VR-1541F). App package
`com.amd.wirelessgvr`.

---

## HelloResponse acceptance (open)

We unicast-reply; client appears to parse then:

- ReliveVR app exits / user describes it as “kills the app”, **or**
- Client keeps rediscovering every ~6s from new ports.

Likely causes still under test:

- Wrong/missing JSON fields or types (ChannelsSupported format, Transports,
  Options shape).
- Wrong type byte (try 1).
- Need additional post-Hello handshake immediately.
- Crash may be unrelated GoogleVrCore calibration abort — need AMD-tagged
  logcat at the moment of failure.

Probe supports:

```bash
RELIVEVR_STYLE=minimal|echo|full nix run .
RELIVEVR_TYPE=0|1 RELIVEVR_STYLE=minimal nix run .
```

---

## JSON keys recovered from FromJSON / rodata

### HelloResponse
ProtocolVersion, ProtocolMinVersion, MaxDatagramSize, DeviceID, Options,
ServerName, ChannelsSupported, Transports

### StartRequest
DisplayModel, DisplayWidth, DisplayHeight, FrameRate, Bitrate-related,
InterpupillaryDistance, AspectRatio, SeparateEyeProcessing, VideoCodec,
NonLinearScalingSupported

### VideoData (per-frame metadata)
ptsSensor, ptsServerLat, ptsEncoderLat, pts, cmpFrmSize, frmType, encType,
ptsSend, frameNum

### VideoInit
CodecID, NonLinearScaling, DisplayWidth/Height, Bitrate, …

### AudioInit
SampleRate, Format, PTS, …

### Other symbols
HelloRequest, HelloRefused, StopRequest, UpdateRequest, VideoForceIDR,
DeviceEvent, TrackableDeviceCaps, StatLatency, VirtualWall, …

---

## Channel enum

`IsChannelSupported(Channel ch)` → `*(uint8_t*)(this + 97 + ch)`.

Channel ∈ {0..7} at least. Role per channel unknown.

---

## Binary video path (static only)

```
Communicator(VideoReceiverCallback, AudioReceiverCallback, …)
  → DisplayPipeline::SubmitSPSPPS / SubmitFrame
  → MediaCodecDecoder::SubmitSPSPPS / SubmitInput
```

Logs: `decoder start L eye pts=%lld ID=%lld` (and R eye).  
MIME: `hevc`, `video/`, `audio/mp4a-latm`.  
Separate-eye + non-linear scaling supported.

Wire format for NAL/AAC on a Channel: **unknown**.

---

## Important classes / symbols

| Symbol | Role |
|--------|------|
| AWVRCreateClient | Client entry |
| Communicator | Discovery, connect, channelled send/recv |
| FlowCtrlProtocol | Fragmentation |
| StreamFlowCtrlProtocol::PrepareMessage(Channel,…) | Stream send |
| ServerDiscoverySession | Client discovery session |
| DiscoveryClient | Discovery helper |
| Command::ParseBuffer | type + JSON |
| HelloRequest / HelloResponse / HelloRefused | Discovery messages |
| ServerParametersImpl | Parsed server advert |
| MediaCodecDecoder | Android decode |
| DisplayPipeline | SubmitSPSPPS, SubmitFrame, sensors |
| SensorEngine::Pose / ControllerState | Tracking |
| DaydreamController | Daydream input mapping |
| Motor::StartDiscovery / DiscoverServers | App-level discovery |

---

## What a fresh session should do next

1. Bisect HelloResponse with RELIVEVR_STYLE / RELIVEVR_TYPE until the
   headset **stops rediscovering** without dying.
2. Capture AMD logcat during a kill:
   `adb logcat -d | grep -iE 'amd|wirelessgvr|awvr|Hello|Abort|signal'`.
3. After accepted Hello, capture StartRequest / next packets (tcpdump -X).
4. Map Channel IDs (hook or traffic after session starts).
5. RE binary video framing (SubmitInput path + any length-prefix logic).
6. DeviceEvent / pose JSON layout for controller feedback.

---

## Probe usage

```bash
nix build          # result/bin/relivevr-server
nix run            # listen + announce + reply
nix develop        # RE shell (rustc, radare2, scapy, tshark)

RELIVEVR_STYLE=minimal nix run .
RELIVEVR_TYPE=1 RELIVEVR_STYLE=echo nix run .
```

Listens on `0.0.0.0:1235`, broadcasts Hello every 2s, unicasts reply to
type 0/1 single-fragment probes. Filters its own announces by DeviceID
substring `relivevr-linux-probe`.

## HelloResponse::FromJSON crash (SIGSEGV) — root cause

Live tombstone:

```
pc … HelloResponse::FromJSON+716
← Command::ParseBuffer
← ServerDiscoverySession::OnCompleteMessage
fault addr 0x0
```

At `FromJSON+716` (VA `0xa5658`) the code does:

```
x0 = node->lookup(key);   // key built on stack
ldr x8, [x0]              // NO null check → crash if key missing
```

Stack key reconstruction + preceding lookups show **required** fields
(no null check before use):

| Key              | Store offset in HelloResponse object |
|------------------|--------------------------------------|
| MaxDatagramSize  | +104 (w)                             |
| **DatagramSize** | +108 (w)  ← crash site key           |
| **Port**         | +112 (h)                             |

`DatagramSize` is distinct from `MaxDatagramSize` (rodata has both as
separate strings; second is substring construction `"Datagram"+"Size"`).

**Fix:** HelloResponse JSON must include at least:

```json
"MaxDatagramSize": 65507,
"DatagramSize": 65507,
"Port": 1235
```

(values matched to client request / default port; may need tuning).

Optional keys (ChannelsSupported, Transports, Options, ServerName, DeviceID,
ProtocolVersion*) may still matter for session setup but are not the
immediate null-deref.


## Type 7 after Hello (live 2026-10-09)

With DatagramSize+Port present, client no longer crashes. Sequence:

1. type=0 HelloRequest (broadcast)
2. our HelloResponse (unicast, type 0)
3. ~5ms later type=7 + same HelloRequest JSON (unicast from new port)
4. ~10s later type=0 again (still rediscovering)

Type byte is likely `SERVICE_OP_CODE`. Values seen: 0 and 7.
Probe now logs full JSON for any type with `{` body and replies to
types 0, 1, and 7 (echoing 7 when request was 7).

