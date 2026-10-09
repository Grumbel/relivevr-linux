# Reverse-Engineering Notes (libwirelessvr-lib.so + APK)

## Binary
- Package: `com.amd.wirelessgvr` 1.0.13
- Native: `lib/arm64-v8a/libwirelessvr-lib.so` (~1.7 MiB)
- Built with Android NDK (clang 8.0.7), AMF 1.3-dev references in paths.
- C++ namespace: `awvr`, plus `amf`, `Comm`, `SensorEngine`, etc.
- Entry: `AWVRCreateClient`
- Java thin wrapper: `com.amd.wirelessgvr.WirelessHMDClient` (JNI methods listed in strings).

## Key classes (from demangled symbols)
- `Communicator` – discovery, connect, channelled send/recv.
- `awvr::FlowCtrlProtocol` – fragmentation:
  - `Fragment` (ParseFromBuffer, ctor with seq / sizes / flags)
  - `Buffer` (AddFragment)
  - `FragmentMessage`, `ProcessFragment`, `SendNextMessage`, `PurgeStaleBuffers`
- `awvr::StreamFlowCtrlProtocol` – higher-level for streams (PrepareMessage taking Channel)
- `awvr::AWVRClientImpl`, `AWVRClientSessionImpl`, `AWVRDatagramClientSessionImpl`, `AWVRStreamClientSessionImpl`
- `MediaCodecDecoder` / `MediaCodecDecoder::Decoder` – `SubmitSPSPPS`, `SubmitInput`, per-eye PTS.
- `SensorEngine::Pose`, `ControllerState`, `OEMPoseData`
- `DaydreamController`, `DaydreamVRRenderer`, `DaydreamVRDisplayPipeline`
- Settings via AMF property storage + JSON parser.
- `ServerDiscoverySession`, `DiscoveryClient`, `Command::ParseBuffer`

## FlowCtrlProtocol::Fragment header (RE'd from disassembly)

**15-byte header, multi-byte fields big-endian**, payload follows immediately.

```c
// From Fragment ctor and ParseFromBuffer / FragmentMessage
struct FragmentHeader {
    uint16_t seq;        // +0  BE, incrementing sequence number
    uint32_t field2;     // +2  BE  (appears to be total message size or ID-related)
    uint32_t offset;     // +6  BE  (byte offset of this fragment within the full message)
    uint32_t length;     // +10 BE  (payload length of *this* fragment)
    uint8_t  flags;      // +14     (last-fragment? channel? observed as parameter)
    // uint8_t payload[length];  starts at offset 15
};
```

- `ParseFromBuffer` requires size >= 16 and `size == length + 15`.
- On success stores the raw buffer pointer and a ownership flag.
- Constructor allocates `length + 15`, writes the BE fields, then memcpy's the payload.
- Sequence is maintained per FlowCtrlProtocol instance and incremented on each FragmentMessage.

This is sufficient to parse every UDP datagram that uses the flow-control layer and to construct valid fragments.

## Discovery path
- `ServerDiscoverySession::OnCompleteMessage` looks at first payload byte after reassembly:
  - 0 → treat as discovery request, call `Command::ParseBuffer`
  - 1 → other handling
- Uses a fixed-size buffer related to 1480 (0x5c8) – likely max discovery payload or MTU-related.
- Discovery is built on top of the same DatagramClientSessionFlowCtrl + FlowCtrlProtocol.

## Observed strings / constants
- Port-related: `StartPort`, `EndPort`, `DiscoveryTimeout`, `EnableDiscovery`
- Transports: `UDP` / `TCP` (via `CTCP`, `Transports`, `Network`)
- Video: `VideoCodec`, `VideoCodecs`, `hevc`, `video/`, `NonLinearScalingSupported`, `Bitrate`, `FrameRate`
- Audio: `audio/mp4a-latm`
- Logging: `decoder start L eye pts=%lld ID=%lld`, `decoder start R eye pts=...`
- Path: `D:\dev\stg\AMF-1.3-dev\tools\src\Android\WirelessGVR\...`
- `ProtocolVersion`, `ProtocolMinVersion`
- `ChannelsSupported`

## What is still missing (high value)
1. Exact meaning of `field2` and `flags` in the fragment header.
2. Structure of the reassembled `Command` (after FlowCtrl).
3. Channel ID values and Command opcodes / message types.
4. How a video frame is turned into one or more messages (NAL length prefix? SPS/PPS out-of-band? PTS/DTS?).
5. Pose serialization format.
6. Full session setup handshake after discovery (capabilities exchange, codec negotiation, etc.).

## Status vs. "simple tool to connect"
- **Yes, enough for a useful probe / logger / discovery responder.**
  We can bind UDP 1235, parse every fragment, log the header fields, reassemble multi-fragment messages, and (with a bit more work on the Command parser) reply to discovery broadcasts.
- **Not yet enough for a full video session.**  We still need the post-discovery handshake, channel numbers for video, and the exact video payload format before the headset's MediaCodec path will accept frames.

## Recommended RE continuation
- Continue disassembly of `Command::ParseBuffer` and the discovery reply path.
- Look for switch tables on the first few bytes after the fragment header / after reassembly.
- Capture live traffic if a Windows + headset setup becomes available.
