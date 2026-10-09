# ReliveVR Protocol Notes

## Discovery
- Default: UDP broadcast, port **1235**.
- Override via client `app.settings` (`EnableDiscovery`, `Server` = `UDP://ip:1235` / `TCP://...`).

## Transport / Fragmentation
15-byte header, multi-byte fields **big-endian**:

```c
struct FragmentHeader {
    uint16_t seq;        // +0
    uint32_t field2;     // +2  (tentative: total size / msg id)
    uint32_t offset;     // +6  fragment offset
    uint32_t length;     // +10 this fragment payload length
    uint8_t  flags;      // +14
    // payload[length] starts at +15
};
```
Total packet size == length + 15. Multi-fragment reassembly by seq/offset/length.

## Control plane
After reassembly:

```
uint8_t type;     // 0 = Hello / discovery family
char    json[];   // AMF JSONParser
```

### Hello (discovery)
Keys in HelloResponse:
- ProtocolVersion, ProtocolMinVersion
- MaxDatagramSize, DeviceID, Options, ServerName
- ChannelsSupported (≤8 bools → Channel 0..7 table)
- Transports (e.g. ["UDP"])

### Session start (StartRequest)
JSON keys recovered:
- DisplayModel, DisplayWidth, DisplayHeight
- FrameRate, Bitrate
- InterpupillaryDistance, AspectRatio
- SeparateEyeProcessing, VideoCodec, NonLinearScalingSupported

### Video metadata (VideoInit / VideoData)
These are **JSON control messages**, not the compressed bitstream:

**VideoInit**: CodecID, NonLinearScaling, DisplayWidth/Height, Bitrate, …
**VideoData** (per-frame metadata): ptsSensor, ptsServerLat, ptsEncoderLat, pts,
  cmpFrmSize, frmType, encType, ptsSend, frameNum

### AudioInit
SampleRate, Format, PTS, …

### Other control messages
StopRequest, UpdateRequest, VideoForceIDR, DeviceEvent (pose), TrackableDeviceCaps, …

## Binary video / audio path
- Communicator is constructed with `VideoReceiverCallback` and `AudioReceiverCallback`.
- DisplayPipeline::SubmitSPSPPS / SubmitFrame / MediaCodecDecoder::SubmitInput
  receive raw buffers (Annex-B or length-prefixed NALs + AAC frames).
- The exact Channel ID and on-wire framing of the binary video stream are still unknown;
  the JSON VideoData/VideoInit messages likely travel on a control channel while the
  heavy bitstream uses a dedicated binary channel.

## Channel
`Command::Channel` = small integer (0–7).  
`ChannelsSupported` JSON bool array populates the support table used by IsChannelSupported.

## Probe status
- Parses fragment headers.
- Prints type + JSON for control messages.
- Replies to type-0 single-fragment probes with a crafted HelloResponse.
