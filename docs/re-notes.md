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
  - `Fragment` (ParseFromBuffer, ctor with seq / sizes / flags?)
  - `Buffer` (AddFragment)
  - `FragmentMessage`, `ProcessFragment`, `SendNextMessage`, `PurgeStaleBuffers`
- `awvr::AWVRClientImpl`, `AWVRClientSessionImpl`, `AWVRDatagramClientSessionImpl`, `AWVRStreamClientSessionImpl`
- `MediaCodecDecoder` / `MediaCodecDecoder::Decoder` – `SubmitSPSPPS`, `SubmitInput`, per-eye PTS.
- `SensorEngine::Pose`, `ControllerState`, `OEMPoseData`
- `DaydreamController`, `DaydreamVRRenderer`, `DaydreamVRDisplayPipeline`
- Settings via AMF property storage + JSON parser.

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
1. Exact layout of discovery request/response packets.
2. FlowCtrlProtocol::Fragment header (size, fields: seq, frag_id, total_frags, flags, channel?).
3. Command / Channel enum values (likely small integers).
4. How a video frame is turned into one or more messages (NAL length prefix? SPS/PPS out-of-band? PTS/DTS?).
5. Pose serialization format.
6. Whether the server must first send a “capabilities” / “hello” message.

## Recommended RE continuation
- Load the .so into Ghidra / r2, focus on:
  - `FlowCtrlProtocol::Fragment::ParseFromBuffer`
  - `Communicator::DiscoverServers` / `OnServerDiscovered`
  - `MediaCodecDecoder::SubmitInput` / `SubmitSPSPPS`
  - Any switch on channel ID near `OnMessageReceived`
- Capture live traffic if possible (Windows Adrenalin + supported headset).
- Decompile the Java side (jadx) for settings keys and any remaining constants.

