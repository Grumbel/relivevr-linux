# TODO / Handoff

## Current tip
- type 7 = SERVICE_OP_CODE_HELLO_DIRECT (logcat confirmed).
- Discovery works; HELLO_DIRECT still fails QueryParameters (~10s timeout).
- Probe: type-7 → Full style + response type 0 by default.

## Next experiments
```bash
# default: Full + type 0 for HELLO_DIRECT
nix run .

RELIVEVR_TYPE=7 RELIVEVR_STYLE=full nix run .
RELIVEVR_TYPE=0 RELIVEVR_STYLE=full nix run .
RELIVEVR_TYPE=1 RELIVEVR_STYLE=full nix run .
```

Watch logcat for success vs "Failed to connect".

## After connect works
- Capture StartRequest / session packets
- Channel map + video

## Bundle
`relivevr-linux-013.1-hello-direct-…`
