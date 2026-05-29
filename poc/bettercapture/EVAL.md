# BetterCapture Evaluation

**Repo:** https://github.com/jsattler/BetterCapture
**Eval date:** 2026-05-28
**Commit at eval:** `cf663fb` (HEAD of `main`)

## TL;DR

BetterCapture is **a polished, MIT-licensed SwiftUI menu-bar GUI app**. It is
**not a CLI**, exposes **no automation surface** (no CLI flags, no URL scheme,
no AppleScript dictionary, no scripting bridge), and the core
ScreenCaptureKit code is **tightly coupled to the GUI** via
`SCContentSharingPicker.shared` (which requires a user click) and an
`@Observable` `SettingsStore` driven by SwiftUI. As a CLI candidate it is a
**non-starter**. As a forkable library it is **only marginally useful** —
the ScreenCaptureKit + AVAssetWriter plumbing is high quality but would need
substantial surgery to detach from the picker.

## Repo structure

```
BetterCapture.xcodeproj         <- Xcode project, single GUI target
BetterCapture/
  BetterCaptureApp.swift        <- @main SwiftUI App, MenuBarExtra
  Info.plist                    <- LSUIElement = true (menu bar app)
  BetterCapture.entitlements    <- sandboxed
  Model/                        <- SettingsStore (748 LOC, UserDefaults)
  Service/                      <- CaptureEngine, AssetWriter, etc.
  View/                         <- SwiftUI views
  ViewModel/RecorderViewModel.swift  <- @Observable, drives everything
BetterCaptureTests/             <- one test file (SettingsStoreTests)
website/                        <- marketing site
```

- **No `Package.swift`** — not a Swift Package, cannot be added as a SPM
  dependency.
- **One Xcode target** (`BetterCapture`) plus a test target. No CLI target,
  no library target, no XPC helper.
- Dependencies: Sparkle 2.9.0 (auto-update), sindresorhus/KeyboardShortcuts
  2.4.0.

Core capture code is ~3,400 LOC of Swift split across
`CaptureEngine.swift` (390), `AssetWriter.swift` (592), `SettingsStore.swift`
(748), and `RecorderViewModel.swift` (501).

## Build experience

Trivial on Xcode 16 / Swift 6.2 / macOS 15:

```bash
xcodebuild -project BetterCapture.xcodeproj -scheme BetterCapture \
  -configuration Release -derivedDataPath ./build CODE_SIGNING_ALLOWED=NO build
```

`** BUILD SUCCEEDED **` on first try. Sparkle and KeyboardShortcuts resolved
from SPM automatically. Output: `build/Build/Products/Release/BetterCapture.app`.

## Did you produce a recording?

**No.** I deliberately did not launch the GUI to drive an interactive
recording — the brief is about CLI fitness, and the capture flow cannot be
started without a user clicking through `SCContentSharingPicker`. Even if I
scripted a click via `osascript`/`cliclick`, the resulting artifact would say
nothing about CLI suitability.

Codec/quality is configurable through the Settings window (ProRes 422/4444,
HEVC, H.264, optional HDR, optional alpha, native vs logical resolution),
but there is no way to specify any of this from outside the GUI.

## Automation surface

| Surface | Present? |
|---|---|
| CLI subcommand / flags | No. Binary launches the GUI; `--help` does nothing. |
| URL scheme (`CFBundleURLSchemes`) | No. Verified absent in `Info.plist`. |
| `open -a BetterCapture --args ...` | No effect — app ignores args. |
| AppleScript dictionary (`sdef`) | No. |
| Scripting Bridge / `NSAppleScript` hooks | None in source. |
| Distributed Notifications / XPC entry | None. |
| Global hotkeys (recording toggle) | Yes, via `KeyboardShortcuts` — user-bound only, not scriptable. |

The **only** non-GUI control is whatever you can synthesize with `osascript`
keystrokes against the menu-bar extra, which is brittle and still requires
the user to have launched the GUI and granted Screen Recording permission.

## Maintenance signals

- 166 total commits, all within the last ~16 months.
- Very active: 14 tagged releases on the path `v2026.1.0` → `v2026.3.1-beta.1`,
  most recent `v2026.3.1-beta.1` on 2026-03-29 (~2 months before eval).
- Single primary maintainer (jsattler); README discourages unsolicited
  feature PRs.
- Sponsored by Recall.ai. Lists QuickRecorder/Azayaka as inspirations
  (both also GUI menu-bar apps).
- Tests cover only `SettingsStore`. `CaptureEngine`/`AssetWriter` have no
  unit tests.

## Reusability as a library

If you wanted to fork the capture code into a CLI, you would need to:

1. Remove the dependency on `SCContentSharingPicker.shared` and replace it
   with `SCShareableContent.current` + your own display/window selection
   logic (the picker is the only way `CaptureEngine` learns about a
   `SCContentFilter` today — see `CaptureEngine.swift:282-295` and the
   `SCContentSharingPickerObserver` conformance).
2. Replace `SettingsStore` (a 748-LOC `@Observable` UserDefaults wrapper used
   pervasively by `CaptureEngine.startCapture`, `createStreamConfiguration`,
   and `AssetWriter`) with a plain struct.
3. Strip `@MainActor` from `CaptureEngine` so it can run from a non-GUI
   `main()`.
4. Detach `AssetWriter` from the engine's delegate/queue topology — feasible,
   it's already `Sendable` and split out cleanly.

In other words: `AssetWriter.swift` is the most directly liftable file.
`CaptureEngine.swift` is structurally useful as a reference but not as a
drop-in. You would end up rewriting ~40-50% of it.

## Verdict

**GUI competitor, not a CLI and not a practical library.**

- As a CLI candidate: 0/10. Nothing to invoke headlessly.
- As a forkable library: 3/10. Code quality is good, but the GUI coupling
  (picker, SwiftUI settings, MainActor) means you'd be doing a port, not a
  reuse. The genuinely valuable knowledge is the ScreenCaptureKit/AVAssetWriter
  configuration choices — worth reading, not worth depending on.
- As a GUI competitor to QuickRecorder/Azayaka: actively maintained, well
  engineered, with HDR, ProRes, alpha, native-resolution capture — strong.

If the goal is a Mac video recording CLI, BetterCapture is the wrong
starting point. Better leads: `screencapture(1)` (built-in, limited),
ScreenCaptureKit sample code from Apple, or starting fresh from
`SCShareableContent` + `AVAssetWriter`. The Azayaka project (linked in
their README) is closer to a hackable minimal recorder, though also GUI.
