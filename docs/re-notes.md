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
