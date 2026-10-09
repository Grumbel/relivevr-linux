# TODO / Handoff

## Status
- Video solid.
- Empty type-4 `{}` = Daydream button / system event (not CStartSensor).
- Pose path mapped in SO: SensorThread → QueryAndSendSensors → SendSensorData.
- SensorThread never active in our sessions; CStartSensor does not help.

## Test
```bash
nix run .    # no START_SENSOR
# Press Daydream button → expect type-4 JSON {}
# Move head / trackpad → expect nothing extra until SensorThread active
```

## Next
1. Windows ReliveVR packet capture (ground truth service messages)
2. On-device hook SetActive / QueryAndSendSensors with official stack
3. GOT caller resolution for StartCommunications / SensorThread::Start
4. Clean VideoInit spray

## Notes
- Base: 6813f93

## Bundle
`/home/workdir/artifacts/relivevr-linux-019.1-re-sensorthread-type4-button-6813f93.bundle`
Apply: `git pull /path/to/relivevr-linux-019.1-re-sensorthread-type4-button-6813f93.bundle HEAD`
Tip: fc5416e
