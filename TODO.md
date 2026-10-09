# TODO / Handoff

## Current tip
- Live HelloRequest captured from VR-1541F; responder refined (field2, Options shape).

## Confirmed from live traffic
- ProtocolVersion/MinVersion = 1
- field2 = payload length (single-fragment)
- Client HelloRequest JSON shape (DeviceID, MaxDatagramSize, Options.DeviceType, versions)
- Options uses AMF-variant encoding in JSON

## Open
1. Does the client accept our HelloResponse? (still re-probing every ~6s — may need
   response shape tweaks: ChannelsSupported format, Transports, Options, or type byte)
2. Capture what happens after a successful Hello (StartRequest / channel setup)
3. Binary video framing + Channel roles
4. Pose / DeviceEvent JSON

## Next
- Watch whether client stops rediscovering after our refined reply
- If not: try alternate response shapes (ChannelsSupported as ints, type=1, etc.)
- tcpdump -X the full exchange while client is connecting
