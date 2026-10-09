# TODO / Handoff

## Current tip
- Connect path only accepts **type 0** as HelloResponse success.
- Type 7 request (HELLO_DIRECT) must get type 0 reply (Full JSON).
- Bundle: 014.1

## Test
```bash
# unset RELIVEVR_TYPE if set
unset RELIVEVR_TYPE
RELIVEVR_STYLE=full nix run .
# expect: type=7 request → reply type=0 style=Full
# logcat should NOT show Failed to connect after 10s
```

## Next after connect succeeds
- StartRequest / session packets
- Video path
