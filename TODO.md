# TODO / Handoff

## Status
- **083.1:** Documented Windows `dumpsmall.pcapng` in `docs/windows-pcap.md`
  (VideoInit HEVC, VideoData field roles, pts timeline, ptsSensor = exact pose
  time, ptsSend ≠ pts). No code change.
- Code tip remains 082.1 pcap field alignment (ptsSend fix, no bad connect bursts).
- Open: sensor pts never resolves on device (`prev=0`); need Linux-session pcap
  or HEVC + sustained fps test — not another pts multiplier.

## Test
```bash
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=openh264 cargo run
# bit-closer to dump:
RELIVEVR_VIZ=1 RELIVEVR_ENCODER=hevc cargo run
```

## Bundle
`/home/workdir/artifacts/relivevr-linux-083.1-docs-windows-pcap-f993e2b.bundle`
