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
