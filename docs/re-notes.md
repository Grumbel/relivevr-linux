# Reverse-Engineering Notes

APK: `com.amd.wirelessgvr` 1.0.13 — `libwirelessvr-lib.so` arm64.

## Milestone: connect works (2026-10-09)

Type-0 HelloResponse after HELLO_DIRECT → QueryParameters **succeeded**.
Client sends StartRequest (type 3) + device caps (type 5). Headset goes past
connection UI to black screen (no video yet).

MediaCodec error: `AMediaCodec_createDecoderByType(video/)` — codec MIME empty
until VideoInit provides `video/avc`.

## Opcode map (confirmed)

| Value | Meaning |
|------|---------|
| 0 | SERVICE_OP_CODE_HELLO — response that completes connect |
| 1 | refused |
| 3 | StartRequest (display/video params) |
| 5 | TrackableDeviceCaps-style device JSON |
| 7 | SERVICE_OP_CODE_HELLO_DIRECT — connect request |

## Crash fixed earlier

HelloResponse::FromJSON+716 null-deref without `DatagramSize` / `Port`.

## Next RE / implement

1. Acknowledge or process StartRequest (type 3) — may need a response opcode.
2. Send VideoInit JSON with CodecID / MIME `video/avc` (and resolution).
3. Send SPS/PPS + IDR frames on the video channel (framing TBD).
4. Pose / DeviceEvent path for controllers (type 5 inputs listed).
5. Why client re-runs discovery every ~10s (session keepalive / missing ACK).

## Probe

```bash
RELIVEVR_STYLE=full nix run .
# type 0 discovery → HelloResponse type 0
# type 7 HELLO_DIRECT → HelloResponse type 0 Full
# logs type 3 StartRequest and type 5 caps
```

## VideoInit (static RE)

FromJSON @ 0xe2ce0:
1. GetInt32 Width
2. GetInt32 Height
3. GetString CodecID
4. GetBool NonLinearScaling

Always returns success (1). No null-check crashes on missing keys (unlike Hello).

MediaCodec: `AMediaCodec_createDecoderByType("video/" + CodecID)` — empty CodecID
produced the observed `video/` failure.

StartRequest ctor stores type byte **3** at object+8 (matches live).

Probe: on type 3, emit VideoInit with type `RELIVEVR_VIDEOINIT_TYPE` (default 2).

## MediaCodec empty MIME timing (live)

`AMediaCodec_createDecoderByType(video/)` fires ~30ms after HELLO success,
**before** StartRequest (~150ms). Init builds `"video/" + codec`; empty codec
at connect produces `video/`. VideoInit should ReInitDecoder with CodecID=avc
→ `video/avc`. Type byte still unconfirmed (default 2).

Probe also sends VideoInit immediately after HELLO_DIRECT reply.

## UpdateRequest type = 6

`UpdateRequest::UpdateRequest(float)` stores type byte **6** at object+8.

## VideoInit receive still open

No constructor with fixed type found for VideoInit (only FromJSON). Client may
dispatch via channel+type table rather than a fixed SERVICE_OP. Probe sends
multiple type×CodecID combinations after StartRequest.

## StreamFlowCtrl 7-byte header (PrepareMessage @ 0x9e3f0)

```
str BE(payload_len) at +0
strb channel at +4
sturh BE(stream_seq) at +5
memcpy body at +7
```

Probe now emits VideoInit both plain and stream-framed (channels 0–3, types
2/4/8 + configured).

## VideoInit works (2026-10-09)

`ReInit(avc)` succeeds with 1440×1440 output format. Which of plain vs stream
and which type/channel triggered it is not isolated yet (spray sent many).
Narrow later; priority is binary frame path.

Session times out ~10s without frames → rediscovery.

## Channel discovery (2026-10-09)

Fragment flags → Buffer+56 channel. Jump table channels 0,1,2,7 only.

## OnFrameReceived @ 0xdc388

- First byte 1: strlen from byte1, ParseBuffer as VideoData, remainder = NALs
- First byte 0: ParseBuffer (VideoInit?), binary after string
- Other: error path

## Next live test

Send flags=1 frames with VideoData JSON + Annex-B. Watch for OnFrameReceived /
SubmitSPSPPS / SubmitFrame in logcat. Current NALs are placeholders.

## Real H.264 IDR embedded (2026-10-09)

Generated with:
`ffmpeg -f lavfi -i color=c=blue:s=1440x1440 … -profile:v baseline`
Access unit ~6.8 KB. Sent as VideoData body on channel 1 after VideoInit.

## Frames decoded (2026-10-09)

`Decoder lag around frame #3` after channel-1 VideoData frames. Placeholder
170B NALs were enough to exercise the path; real IDR is in tip 024+.

Type 9 from client after frames — no JSON, single byte. Respond with IDR.
Continuous ~60 fps stream armed after StartRequest to avoid 10s rediscovery.

## Stereo (2026-10-09)

Full mono blue → left only. SBS blue|red embedded for next test.

RenderEye(eye, fbo, x, y, w, h, flag) — viewport per eye from GVR.

## Pose path (stub)

SendSensorData / SendControllerData on Communicator. DeviceEvent on channel 7.
Defer until image path is stable at 30 fps without decoder-full spam.

## Dual decoder slots (2026-10-09)

OnFrameReceived: `orr frmType,#2; cset ne` indexes decoder pointer table.
frmType 0 → slot 0 (left), frmType 1 → slot 1 (right).

String: `decoder both eyes ready pts=%lld ID=%lld`.

User: left saw full SBS blue|red split, right black → only left decoder fed.

## Per-eye colour verification (2026-10-09)

Both decoder slots confirmed live after dual frmType feed.

Next verification step: independent solid colours —
- frmType 0 (left)  → 1440×1440 solid blue IDR
- frmType 1 (right) → 1440×1440 solid red IDR

Generated with:
```
ffmpeg -f lavfi -i color=c=blue:s=1440x1440:d=1 -frames:v 1 \
  -c:v libx264 -profile:v baseline -level 4.1 -pix_fmt yuv420p \
  -bsf:v h264_mp4toannexb -f h264 blue_idr.h264
# same for red
```

If left is blue and right is red, SeparateEyeProcessing + dual-slot path is
fully understood. Pose work can begin after this confirmation.

## Colour verification confirmed (2026-10-09)

User: left eye solid blue, right eye solid red.

SeparateEyeProcessing + dual MediaCodec slots fully understood and driven
from the Linux probe. Video path is good enough for static / keyed images.
Next functional gap is pose / controller input (channel 7).

## Complex pattern (2026-10-09)

Replaced solid colour IDRs with labelled grids for spatial verification:

```
ffmpeg -f lavfi -i "color=c=0x003366:s=1440x1440:d=1" \
  -vf "drawgrid=w=90:h=90:t=2:c=0x00ccff@0.9,drawtext=text='LEFT':..." \
  -c:v libx264 -profile:v baseline -pix_fmt yuv420p -bsf:v h264_mp4toannexb \
  -f h264 left_idr.h264
# analogous for RIGHT (0x660033 / 0xff66cc)
```

Sizes ~19–21 KB per IDR AU. Still sent as full IDR every ~60 frames; tiny
P-frame placeholders between.

## Channel-aware demux + pose probe (2026-10-09)

Receive path now:

1. Parse 15-byte FlowCtrl fragment header.
2. `channel = flags` (byte 14).
3. Optionally peel StreamFlowCtrl 7-byte header
   (`u32 BE len | u8 channel | u16 BE seq | body`).
4. Route:
   - ch 0 + JSON → existing service handlers (Hello, StartRequest, caps, …)
   - ch 7 → DEVICE_EVENT log (hex + f32 LE/BE)
   - other binary → hex + f32 preview

Goal of next live session: move head + Daydream controller and record
packet sizes / first 8–16 floats to recover:

- HMD orientation (quaternion or matrix)
- HMD position (if 6DoF; Daydream/Mirage Solo may be 3DoF rotation-only)
- Controller orientation + trackpad axes + buttons

Known symbols (static): `SendSensorData`, `SendControllerData`,
`SensorEngine`, `ControllerState`, `Pose`, `OEMPoseData`,
`DaydreamController`, `TrackpadEmulator`.

## Live session 2026-10-09 15:47 (user log)

Connect sequence confirmed:

1. type 0 HelloRequest (broadcast) → HelloResponse
2. type 7 HELLO_DIRECT → HelloResponse Full
3. type 5 HMD caps `/hmd` DoF=true
4. type 3 StartRequest 1440×1440 avc 60Hz SeparateEye+NLS IPD=0.064
5. VideoInit spray + LEFT/RIGHT IDRs → continuous stream armed
6. type 5 controller caps `/ctrlRight` Daydream
7. type 9 single-byte binary, repeating (force-IDR / keepalive)

**No channel-7 DeviceEvent and no multi-byte binary pose** during this capture.

Decoder lag ~31s in logcat — PTS was frame_num×16666µs at 30fps send rate
(under-advancing vs wall clock). Probe now uses wall-clock µs from stream origin.

Hypothesis: client only emits SendSensorData after the Windows OpenVR driver
advertises a ready tracked device, or after an unobserved server→client
“enable sensors” message. Next: static RE of SensorEngine / when
SendSensorData is called; try ACKing caps; try empty channel-7 probe.

## Type 9 flood (2026-10-09)

Client sends type=9 body=1 at very high rate (seq advances multiple times per
10ms — order of 100–250/s). Treating each as force-IDR caused dual ~20KB IDRs
per event and severe DisplayPipeline lag.

Correct handling: ignore payload; keep `video_client` armed; continuous
stream supplies frames. Summarise count on announce tick.

## Healthy video 15:52 (2026-10-09)

After stopping type-9 IDR flood + wall-clock PTS + 60 fps:

- Decoder lag **7–11 ms** (was 17–31 **seconds**)
- type9 count = 4 at session start, then flat
- ReInit(avc) 1440×1440 succeeds; LEFT/RIGHT patterns display

Type 9 flood was **feedback** from decoder starvation, not steady-state protocol.

Pose still absent. Client may require a server-driven enable we have not found.

## Caps ACK experiment (2026-10-09 15:54)

After each type-5 caps JSON, probe sent:
- type 5 JSON `{"status":"ok"}` on channel 0
- channel 7, type 4, body `{"event":"tracking"}`

Client behaviour unchanged: no sensor/pose packets followed.
logcat only shows connect + ReInit(avc). Pose enable path is elsewhere.

## Caps ACK crash suspicion (2026-10-09)

Tip 013 sent type-5 `{"status":"ok"}` and ch7 type-4 `{"event":"tracking"}`
after each caps message. User observed ReliveVR client dying/restarting
(new PID 10066, Init(avc)+InitAudioDecoder then StartDiscovery).

Same pattern as earlier HelloResponse null-deref: client FromJSON paths are
brittle. **Do not send speculative JSON** for opcodes without a known schema.

Probes removed in tip 015.

## Static RE: libwirelessvr-lib.so (Oculus 1.0.26) + libawvr.so (2.0 beta)

Sources:
- `ReLiveVR-Oculus-1.0.26.apk` (GPUOpen release) → `lib/arm64-v8a/libwirelessvr-lib.so`
- `ReLive-VR.2.0-beta.apk` → `libawvr.so`, `liboculuswirelessvr-lib.so`

Daydream 1.0.13 SO was not re-downloaded this session; Oculus 1.0.26 shares the
same `awvr::` / `Communicator` / `SensorEngine` architecture.

### Pose send path (client → server)

```
Motor::SensorThread::Run
  → Motor::QueryAndSendSensors(long&)
  → Communicator::SendSensorData(Command const&)
  → Communicator::SendControllerData(Command const&)
  → Communicator::SendMessage(Channel, void const*, size)
```

Also: `Communicator::SendServiceData(Command const&)`.

`Motor::SensorThread::SetActive(bool)` gates the thread.
`Motor::ControllerInitializer` brings up controllers.

### Explicit sensor control strings

In `.rodata` next to DeviceEvent field names:

| String | Role |
|--------|------|
| `CStartSensor` | Start-sensor message (likely server→client or local cmd) |
| `StopSensor` | Stop sensor |
| `Message` | DeviceEvent JSON field |
| `type` | DeviceEvent JSON field |
| `width` / `height` | DeviceEvent JSON fields |
| `events` | DeviceEvent JSON field (array?) |
| `id` | device id path |
| `data` | payload |
| `time` / `flags` | metadata |

### DeviceEvent JSON value keys (pose body)

| Key | Meaning (inferred) |
|-----|-------------------|
| `orient` | orientation quaternion |
| `pos` | position vector |
| `orientV` | angular velocity? |
| `posV` | linear velocity |
| `orientA` | angular acceleration? |
| `posA` | linear acceleration |
| `/hmd` | HMD device path |
| `/pose` | pose subpath |
| `/battery` | battery |
| `/ctrlRight` `/ctrlLeft` | controllers |
| `/in/tp/val` `/in/tp/click` `/in/tr` … | input paths (match caps JSON) |

`DeviceEvent::Pose::ToJSON` / `FromJSON` and `AddValue` overloads for Pose,
`vector<float>`, bool, float, long confirm **JSON** encoding (not raw binary).

### Channel map (unchanged)

| Ch | Role |
|----|------|
| 0 | SERVICE |
| 1 | VIDEO |
| 2 | AUDIO |
| 7 | DeviceEvent (type 4 in jump table) |

### Why we see no pose on the wire

Client never calls `QueryAndSendSensors` until `SensorThread` is active.
Activation likely requires:

1. Local decoder/session ready (we achieve ReInit), **and**
2. A `CStartSensor` / equivalent service message we do not send, **or**
3. Some Windows-driver-only handshake.

Probe experiment (opt-in): `RELIVEVR_START_SENSOR=1` sends a minimal
DeviceEvent-shaped JSON with `"Message":"CStartSensor"` on channel 7 after
StartRequest. **May crash client** if schema is wrong — off by default.

### Controller input paths (from SO strings)

```
/in/tr  /in/tp/val  /in/tp/touch  /in/tp/click
/in/sys/click  /in/menu/click
/in/a /in/b /in/x /in/y /in/sys /in/grip /in/js
/out/haptic
```

Profiles: `Oculus6DoF`, `GearVR3DoF`, `OculusGo3DoF`.

## Live CStartSensor 16:03 (2026-10-09)

With `RELIVEVR_START_SENSOR=1`:

1. After StartRequest + IDR, probe sent CStartSensor on ch7 + service type 4.
2. Client replied **type=4 JSON `{}`** (body len 3) ~100ms later — first DeviceEvent
   seen on the wire from client.
3. No `orient`/`pos` stream followed.
4. Rediscovery ~5s later (new UDP ports) — possible soft failure from schema.

Interpretation: client accepted *something* about type-4 DeviceEvent path and
emitted an empty event; sensor thread still not producing poses. Next: delayed
minimal variants on service type 4 only; watch for non-empty type-4 or binary.

## Live CStartSensor variants 16:07 (2026-10-09)

Three delayed variants on service type 4:
- `{"Message":"CStartSensor"}`
- `{"Message":"CStartSensor","type":0}`
- `{"events":[{"id":"/hmd"}]}`

Result: one type-4 JSON `{}` (seq=6) ~9s after connect; no pose fields.
Rediscovery continued. type9 count stayed low (video healthy).

User: Daydream button gives a local reaction; other buttons / motion do not
appear on the wire (expected while SensorThread inactive).

**Conclusion:** CStartSensor probe is not sufficient to activate
`QueryAndSendSensors`. Empty type-4 may be an independent keepalive/ACK.
Stop multi-variant spray; keep single opt-in probe only.

## Deeper SO RE (2026-10-09 evening)

Binary: Oculus 1.0.26 `libwirelessvr-lib.so` arm64 (GPUOpen release).

### Sensor pipeline (confirmed by symbols + disasm)

```
Motor::Connect / StartCommunications
  → (sets up /hmd device paths, FOV, EncoderSize, VideoCodecs, …)
Motor::SensorThread::Start
Motor::SensorThread::Run
  → loop while active-flag byte [SensorThread+0x28] nonzero
  → Motor::QueryAndSendSensors(long&)
       builds DeviceEvent with paths "/hmd", "/pose", "/battery"
       (string ADRPs at QueryAndSendSensors+0x44 / +0x1d4)
  → Communicator::SendSensorData(Command)
  → Communicator::SendControllerData(Command)
  → Communicator::SendMessage(Channel, buf, len)
```

`SensorThread::SetActive(bool)` only manipulates shared_ptr state around
offset +48 — the Run loop samples a **byte at +0x28** (`LDRB`/`CBZ`).

**No direct `BL` to these functions** in the DSO — all calls are PIC via GOT
(`BLR`). Call-graph via static BL xref is empty; relocation slots exist for
`QueryAndSendSensors`, `SetActive`, `Start`, `Run`, `SendSensorData`.

### StartCommunications (0xee6a4)

References JSON/property keys: `HorizontalFOV`, `VerticalFOV`, `EncoderSize`,
`VideoCodecs`, `/hmd`. This is the post-connect “session config / device
announce” path on the **client**, not something we currently trigger beyond
normal Hello/StartRequest/video.

### Empty type-4 DeviceEvent (live)

Client → server, body `4 {}` (type byte + empty JSON), often `seq=6`.

User observation: **Daydream button produces this with or without
`RELIVEVR_START_SENSOR`**. So it is **not** caused by our CStartSensor probe.

Likely: system/UI event (home/app button) converted to a minimal DeviceEvent
with no data elements. Unrelated to `QueryAndSendSensors` (which would carry
`orient`/`pos` under `/hmd`/`/pose`).

### Why pose still absent

`SensorThread` never becomes active in our sessions:

1. `SetActive(true)` never reached, or
2. `SensorThread::Start` never called after `StartCommunications`, or
3. Connect path incomplete vs Windows driver (missing service message after
   video is flowing).

CStartSensor probes only elicited empty `{}` and did not set the active flag.

### Next RE steps

1. Dynamic: Frida/on-device hook of `SensorThread::SetActive` and
   `QueryAndSendSensors` while using **official Windows ReliveVR**.
2. Static: finish GOT/PLT resolution for callers of `StartCommunications`
   and `SensorThread::Start` (who invokes them after Connect).
3. Compare HelloResponse / post-StartRequest service messages from a Windows
   capture vs our probe.

## StartCommunications = client StartRequest builder

`Motor::StartCommunications` (0xee6a4) references the same JSON keys the
client sends in StartRequest:

`HorizontalFOV`, `VerticalFOV`, `EncoderSize`, `VideoCodecs`, `/hmd`,
`DisplayModel`, `DisplayWidth`/`Height`, `Bitrate`, `FrameRate`,
`InterpupillaryDistance`, `AspectRatio`, `SeparateEyeProcessing`,
`VideoCodec`, `NonLinearScalingSupported`, `AudioChannels`,
`AudioChannelLayout`.

So StartCommunications runs on the **client** when connecting and emits
type-3 StartRequest (matches our live sequence). It is not a server opcode.

## Additional protocol types (RTTI / symbols)

| Type (name) | Notes |
|-------------|--------|
| `VideoForceIDR` | FromJSON only; likely related to client type-9 force-IDR demand |
| `UpdateRequest` | ctor takes `float` (bitrate / quality?) |
| `StopRequest` | ctor takes `int` |
| `ProfileNetwork` / Ack / Nack / Response / UpStream / DownStream | bandwidth probing |
| `StreamFlowCtrlProtocol` | fragment reliability (already partially RE'd) |
| `TrackableDeviceDisconnected` | device removal |

Type-9 1-byte packets remain best treated as force-IDR **demand** / keepalive,
not as a payload we must answer with dual-eye IDRs (that caused decoder lag).

## Static call-graph limits

- Vtables for `SensorThread` / `ControllerInitializer` are **zero in the file**
  (filled by RELATIVE relocations at load).
- No direct `BL` to `QueryAndSendSensors` / `SetActive` / `StartCommunications`
  in the DSO — all PIC via GOT/`BLR`.
- Practical activation condition for SensorThread is still unknown without
  dynamic tracing (Frida) or a Windows packet capture.

## Daydream vs GearVR SO

Analyzed SO is **Oculus 1.0.26** (`libwirelessvr-lib.so`) with GearVRRenderer
symbols. User device is Daydream (`AMD WVR Daydream` caps). Same `awvr::`
protocol layer; sensor activation may still differ by HMD backend
(GVR vs GearVR vs OVR). Daydream 1.0.13 APK was not re-fetched this session
(APKMirror/APKPure blocked); Oculus build remains the best available proxy.


## Live pcap (dump.pcapng, 2026-10-09) — pose unlocked on Windows

15 UDP packets, headset `.33` ↔ PC `.51:1235`.

### Sequence

| # | Dir | Content |
|---|-----|---------|
| 0 | C→S broadcast | Hello type 0 |
| 1 | S→C | HelloResponse **ProtocolVersion 2**, ChannelsSupported[4]=true, VideoCodecs **hevc** |
| 2 | C→S | HELLO_DIRECT type 7 |
| 3 | S→C | HelloResponse ProtocolVersion 1, same channels/codecs |
| 4 | C→S | TrackableDeviceCaps `/hmd` type 5 |
| 5 | C→S | StartRequest **VideoCodec=hevc** type 3 |
| 6 | C→S | TrackableDeviceCaps `/ctrlRight` type 5 |
| 7 | S→C | **VideoInit** JSON + HEVC VPS/SPS/PPS NALs (`BitDepth`,`CodecID`,`Viewport`,`ID`,…) |
| 8 | C→S | type 5 `{"Message":"StartSensor"}` |
| 9–14 | C→S | type 4 pose JSON ~60–100 Hz |

### StartSensor is client→server

Not a server probe. Client emits `{"Message":"StartSensor"}` **after** receiving
VideoInit. Then pose DeviceEvents stream.

### Pose DeviceEvent schema (type 4)

```json
{
  "events": [
    {
      "id": "/hmd/pose",
      "data": [{
        "time": 17915786107968154,
        "val": {
          "baseFrmIdx": 17, "frmIdx": 17,
          "orient": [qx,qy,qz,qw],
          "orientA": [0,0,0], "orientV": [0,0,0],
          "pos": [x,y,z],
          "posA": [0,0,0], "posV": [0,0,0]
        }
      }]
    },
    { "id": "/hmd/battery", "data": [{ "val": 1.0 }] },
    {
      "id": "/ctrlRight/pose",
      "data": [{
        "time": …,
        "val": { "baseFrmIdx", "frmIdx", "orient":[4], "pos":[3] }
      }]
    },
    { "id": "/ctrlRight/battery", "data": [{ "val": 1.0 }] }
  ]
}
```

### Header note

16 bytes before JSON; last u16/u32 encodes body length + type
(e.g. type 5 → `…0005`, pose type 4 with ch nibble `…0404`).

### Why our server never got pose

Client never sent `StartSensor` — likely because HelloResponse/VideoInit
differed (channels, codecs, VideoInit shape). Tip aligns Hello + VideoInit
with this pcap.


## cap2.pcapng (2026-10-09) — StartSensor is SERVER → client

2742 packets. After VideoInit (#28 S→C hevc + NALs), packet **#29 S→C**:

```
header … 00 1a 00 05
{"Message":"StartSensor"}
```

type **5**, length 0x1a=26 body. Then client floods `/hmd/pose` + `/ctrlRight/pose`
(~1778 pose packets). Client also sends type 6 `{"FrameRate":…}`.

Earlier dump misread direction on StartSensor. Server must emit it after VideoInit.

## Daydream lens FOV / distortion (2026-10-10)

- HelloResponse Options: HorizontalFOV / VerticalFOV = 1.745 rad ≈ **100°**.
- Measured Daydream View (2017) total FOV ≈ **89°** (sitesinvr).
- Cardboard / GVR coefficients (p' = p (1 + K1 r² + K2 r⁴), tan-angle units):
  - Daydream View v1: K1=0.385, K2=0.593
  - Daydream View v2: K1=0.4331, K2=-0.0856 (reported; second term negative)
- Current server: plain perspective projection, no radial distortion mesh/shader.
  Default encode FOV raised to 90°. Full pre-distortion (barrel) for the lenses
  is still TODO — official Windows server behaviour unknown (may rely on client
  MediaCodec path or send undistorted).

## Hardware encode (2026-10-10)

`src/encode.rs` selects backend via `RELIVEVR_ENCODER`:

| Value | Backend |
|-------|---------|
| `auto` (default) | nvenc → vaapi → qsv → libx264 → OpenH264 |
| `nvenc` / `vaapi` / `qsv` / `x264` | FFmpeg forced |
| `openh264` | software only |

FFmpeg path: RGBA → NV12 → stdin pipe → Annex-B on stdout (`-f h264`, zerolatency / low_delay).
Needs system `ffmpeg` with the chosen encoder. VA-API device: `RELIVEVR_VAAPI_DEVICE`.

## FFmpeg pipe: `-fflags +nobuffer+flush_packets` drops all frames (2026-10-10)

Isolating the exact cmdline from `RELIVEVR_ENCODER=x264` warm-up failures:

| Flags | 1×1440² NV12, stdin left open | after stdin close |
|-------|-------------------------------|-------------------|
| baseline rawvideo→libx264→h264 | **AU in ~100ms** | AU |
| `+ -fflags +nobuffer+flush_packets` | **no data** | **empty** (`No filtered frames for output stream`) |
| `+ -fps_mode passthrough` alone | AU after close / with select | AU |
| mpegts mux, no fflags | AU in ~75ms | AU |

Conclusion: remove `+nobuffer+flush_packets` from the persistent encoder. Low-latency is already covered by `-tune zerolatency`, `-bf 0`, and `-flags low_delay`.

## Viz encode pacing (not the bottleneck)

`ControlFlow::Poll` + `AboutToWait → request_redraw` + `min_dt = 1/target_encode_fps`. Encode runs as fast as GL readback + encoder allow, capped at target fps. No artificial low-fps limit beyond that.
