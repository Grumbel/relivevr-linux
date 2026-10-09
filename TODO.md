# TODO / Handoff

## Current tip
- Client receives our HelloResponse then app dies / rediscovers.
- Responder now has style variants: minimal | echo | full (RELIVEVR_STYLE)
  and type byte override (RELIVEVR_TYPE=0|1).

## Live confirmed
- HelloRequest from VR-1541F (see protocol.md)
- ProtocolVersion=1, field2=payload length, Options AMF-variant JSON
- Unicast reply reaches the client

## Open
1. Find which response shape the client accepts without crashing.
2. AMD logcat around the kill: `adb logcat --pid=$(adb shell pidof -s com.amd.wirelessgvr)`
3. After accepted Hello: StartRequest / session / video path.

## Try
```bash
RELIVEVR_STYLE=minimal nix run .
RELIVEVR_STYLE=echo    nix run .
RELIVEVR_STYLE=full    nix run .
RELIVEVR_TYPE=1 RELIVEVR_STYLE=minimal nix run .
```
