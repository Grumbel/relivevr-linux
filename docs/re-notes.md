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
