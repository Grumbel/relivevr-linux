# TODO / Handoff

## Current tip
- HelloResponse crash fixed (DatagramSize + Port).
- Client proceeds to type=7 message; still rediscovers on type=0 ~10s.
- Probe replies to type 7 as well.

## Working
- [x] No more SIGSEGV on HelloResponse
- [x] Live type=7 observed and documented
- [x] Reply to 0/1/7

## Next
1. Confirm whether type-7 reply stops rediscovery (try styles / type bytes)
2. Full JSON of type 7 in logs; look for extra fields vs type 0
3. tcpdump -X one full 0→reply→7→reply cycle
4. StartRequest / session after handshake settles
5. Video channel

## Bundle
About to cut `relivevr-linux-012.1-type7-…`
