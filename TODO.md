# TODO / Handoff

## Current tip
- Root cause of client SIGSEGV found: missing `DatagramSize` + `Port` in HelloResponse.
- Probe updated to always send them. Bundle about to be cut as 011.1.

## Confirmed
- Crash: HelloResponse::FromJSON+716 null deref on missing DatagramSize lookup
- Required JSON keys: MaxDatagramSize, DatagramSize, Port (and likely versions)

## Next
1. `nix run` with fixed response — confirm app no longer SIGSEGVs
2. If still rediscovering: try styles / type byte; watch for next packet (StartRequest)
3. tcpdump -X after successful Hello
4. Channel map + video framing

## Docs
- protocol.md / re-notes.md updated with crash analysis
