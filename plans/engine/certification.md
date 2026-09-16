# Mobile physical certification

The automated gate builds and executes the release cancellation fixture on an
iOS Simulator and an Android emulator. It does not constitute physical-device
certification. Simulator and emulator reports always retain `physical: false`,
the exact release artifact fingerprint, and the runtime identity that executed
the fixture.

## Automated build and fixture proof

Run both required virtual-device checks from the repository root:

```sh
./scripts/mobile_worker_proof.py \
  --target ios-simulator \
  --output "$HOME/Library/Caches/Battlement/engine-evidence/engine-05-ios.json"

./scripts/mobile_worker_proof.py \
  --target android-emulator \
  --install-android-prerequisites \
  --output "$HOME/Library/Caches/Battlement/engine-evidence/engine-05-android.json"
```

The Android prerequisite option installs the emulator, platform tools, API 36,
and its ARM64 Google APIs system image in the Battlement user cache. Without
those packages, the Android check fails with an explicit prerequisite error.
Use `--build-only` to reproduce either release artifact without claiming that
the fixture ran.

Each passing runtime log must show that rules ran off the Unity thread, nested
destructors completed before worker-stopped, cancellation stayed silent, only
the latest replacement completed, a genuine rules panic was reported as a
failure, the next replacement recovered, and disposal did not join the held
worker. The runner also verifies native exports, packaged architecture, native
runtime linkage, `panic=unwind`, and the final target-specific pass marker.

## Physical-device handoff

Physical certification remains pending. Use the same release artifact and
record these fields separately for an iPhone 17 and a Galaxy S25:

- artifact fingerprint and source revision;
- device model, hardware identifier, OS version, architecture, and build mode;
- installer/signing identity and exact invocation;
- fixture start/end time, full device log, and pass or failure;
- cleanup-before-stopped and silent-cancellation observations;
- genuine-panic failure and subsequent recovery observations;
- launch, sustained-play, memory, frame-time, thermal, and battery captures;
- Hearts launch, pass/card play, menu pause/resume, and restart evidence;
- tester, date, open defects, and attached evidence locations.

Do not copy simulator or emulator results into these fields. The iPhone 17 and
Galaxy S25 functional and sustained-performance checks remain `not run` until
their device logs and captures are attached.
