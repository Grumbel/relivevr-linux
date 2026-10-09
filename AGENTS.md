# ReliveVR Linux / Protocol RE Project

## Goal
Reverse-engineer the AMD ReliveVR (Wireless GVR / Radeon ReLive for VR) protocol so a Linux (NixOS) application can:
1. Discover / connect to a ReliveVR client (headset Android app).
2. Send video frames (initially a static image) that the headset can decode and display.
3. Receive pose / controller input.
4. Later: full SteamVR / OpenXR / monado integration, or merge useful pieces into ALVR / WiVRn.

## Standing rules
- See the session instructions for git-bundle handover, authorship, REUSE/GPLv3, no hacks, etc.
- All work must be continuable from TODO.md + this repo alone.
- Prefer pure ReliveVR-compatible server first; evaluate ALVR/WiVRn merge later.
- Document every non-obvious discovery in `docs/protocol.md` and `docs/re-notes.md`.

## Key prior art / sources
- Official client APKs (this repo contains analysis of `com.amd.wirelessgvr` 1.0.13).
- Public wiki / AMD docs (limited).
- Related open projects: ALVR, WiVRn, Monado, OpenVR, SteamVR.

## Architecture (from prior analysis)
- PC (server): closed-source Windows OpenVR driver + AMF encoder.
- Headset (client): Android app + `libwirelessvr-lib.so` (namespace `awvr`).
- Transport: UDP (default) or TCP, port 1235, with discovery broadcasts and a fragmentation / flow-control layer (`FlowCtrlProtocol`).
- Video: hardware MediaCodec decode (H.264 / HEVC likely).
- Input: pose + controller state sent back on typed channels.

