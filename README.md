# cosmic-comp-traffic

Fork of [pop-os/cosmic-comp](https://github.com/pop-os/cosmic-comp) that adds
macOS-style traffic-light window decorations and a soft focus ring, implements
`zwlr_foreign_toplevel_manager_v1` so third-party docks and taskbars work,
implements `wlr-gamma-control-unstable-v1` so night light clients work, cycles
windows on Alt+Tab without a switcher popup, and ships a ready-to-install
**Arch Linux package for every upstream release tag**.

- Releases: <https://github.com/grigio/cosmic-comp-traffic/releases>
- Base version: upstream tag `epoch-1.10.0` and newer

## What the patches change

| File | Change |
|---|---|
| `src/shell/element/window.rs` | `DefaultDecorations::view` no longer uses libcosmic's `header_bar()`; the titlebar is a themed container with a canvas-drawn traffic light per control (close `#FF5F57`, minimize `#28C840`, maximize `#FEBC2E`, grey `#636366` when unfocused; glyph appears on hover). `SSD_HEIGHT` 36 → 28 so hit-testing matches the new header. `push_render_elements` skips the grey `bg_divider` frame on the focused window, because the focus ring is the outline then. |
| `src/shell/element/window.rs` | `set_activate` stores the `activated` flag *before* queueing the focus event and uses `force_update()` instead of `force_redraw()`, so the traffic lights follow focus immediately. |
| `src/backend/render/shaders/rounded_outline.frag` | Signed-distance ring with `FEATHER = 1.5` logical px of falloff on both edges instead of a hard band. |
| `src/backend/render/mod.rs` | Focus ring element gets 2 px of extra geometry (`pad`) so the glow is not clipped. Other indicators keep `pad: 0`. |
| `src/input/actions.rs` | `System(WindowSwitcher)` / `System(WindowSwitcherPrevious)` cycle the focus through the windows of the active workspace in stacking order instead of spawning a switcher UI — see [Alt+Tab](#alttab). |
| `src/theme.rs` | The theme watcher reloads on any theme change instead of only on a dark/light flip, so `window_hint`, `active_hint` and the `frosted*` flags apply live — see [Live theme reload](#live-theme-reload). |

All six patches live in `patches/common/` and apply cleanly to `epoch-1.10.0`
and `epoch-1.11.0`:

- [`0001-macos-traffic-light-decorations.patch`](patches/common/0001-macos-traffic-light-decorations.patch)
  — the traffic lights and the focus ring described above.
- [`0002-wlr-foreign-toplevel-management.patch`](patches/common/0002-wlr-foreign-toplevel-management.patch)
  — `zwlr_foreign_toplevel_manager_v1`, described below.
- [`0003-three-finger-workspace-swipe.patch`](patches/common/0003-three-finger-workspace-swipe.patch)
  — three-finger touchpad swipes switch workspaces.
- [`0004-alt-tab-window-cycle.patch`](patches/common/0004-alt-tab-window-cycle.patch)
  — in-process Alt+Tab window cycling, described below.
- [`0005-wlr-gamma-control.patch`](patches/common/0005-wlr-gamma-control.patch)
  — `wlr-gamma-control-unstable-v1`, described below.
- [`0006-live-theme-reload.patch`](patches/common/0006-live-theme-reload.patch)
  — theme edits apply without a restart, described below.

## Live theme reload

`watch_theme` reloaded the COSMIC theme only when the dark/light preference
flipped, so every other value in the theme config stayed stale until the
compositor was restarted: accent colours, the `frosted*` flags, and
`window_hint` (the focus ring colour, `active_window_hint()`).

`0006` compares the whole theme instead of just `theme_type`, so editing
`~/.config/cosmic/com.system76.CosmicTheme.<Mode>/v2/*` — for instance
`window_hint` to recolour the focus ring, or `active_hint` to make it thinner —
shows up on the next frame. Pings arrive from the existing cosmic-config
watchers on both the mode config and the dark/light theme configs, so no extra
watcher is needed.

Note that `window_hint` is an `Option<Srgb>`, i.e. RON
`Some((red: 0.545098, green: 0.360784, blue: 0.964706))` for `#8B5CF6`; a hex
string (or `Some(Hex("…"))`) makes `Theme::get_entry` fail and the compositor
silently falls back to the default theme.

## Alt+Tab

Upstream binds `Alt+Tab` / `Alt+Shift+Tab` (and `Super+Tab` / `Super+Shift+Tab`)
to `System(WindowSwitcher)` / `System(WindowSwitcherPrevious)`, which only spawn
whatever command `system_actions` maps those variants to — on a bare session
without that file they are no-ops, and the command is an external popup.

`0004` handles the two variants inside the compositor instead: every press
activates the next window of the active workspace (previous, for the `Shift`
bindings) in stacking order, wrapping around, with no overlay and no helper
process. Windows on other workspaces and minimized windows are skipped, and a
workspace with a single window does nothing.

Because it consumes the `System(WindowSwitcher*)` variants, the external popup
cannot be reached through those bindings any more. To keep it on a different
key, bind e.g. `Super+grave` to `Spawn("cosmic-window-switcher")` (or whatever
switcher is installed) in
`~/.config/cosmic/com.system76.CosmicSettings.Shortcuts/v1/custom`.

The cycle is immediate rather than a popup: each press moves the focus right away,
and the order of the first press is remembered per seat for 3 seconds, so holding
`Alt` and tapping `Tab` walks every window (floating bottom→top, then the tiling
group, then fullscreen surfaces) and wraps around. Without that memory the cycle
would bounce between two windows, because focusing a window raises it and thus
changes the stacking order. A press after the 3 seconds starts a new cycle from
whatever is focused then.

## zwlr_foreign_toplevel_manager_v1

Upstream exposes `ext_foreign_toplevel_list_v1`, which carries title and
app-id but neither toplevel state nor requests, so docks that speak the wlr
protocol (waybar's `wlr/taskbar`, sfwbar, lxqt-panel, noctalia, ...) see no
windows at all on COSMIC ([pop-os/cosmic-comp#3002](https://github.com/pop-os/cosmic-comp/issues/3002)).

The second patch adds the wlr manager (version 3) next to the existing globals,
behind the same `client_not_sandboxed` filter:

| File | Change |
|---|---|
| `src/wayland/protocols/wlr_foreign_toplevel.rs` | New. Mirrors the toplevels tracked by `ToplevelInfoState` to every bound manager (`title`, `app_id`, `output_enter/leave`, `state`, `done`, `closed`) and forwards `activate`, `close`, `set/unset_minimized`, `set/unset_maximized` and `set/unset_fullscreen` to `ToplevelManagementHandler`, so the actions behave exactly like `zcosmic_toplevel_manager_v1`. |
| `src/wayland/handlers/wlr_foreign_toplevel.rs` | New. Implements the handler for `State` and delegates the two protocol objects. |
| `src/wayland/protocols/toplevel_info.rs` | Exposes the outputs a toplevel is on, so the wlr handles can send `output_enter`/`output_leave`. |
| `src/state.rs`, `src/shell/mod.rs` | Creates the global and refreshes the wlr handles from `Common::refresh`, right after the toplevel-info refresh. |

`ext_foreign_toplevel_list_v1` and the COSMIC protocols are untouched; this is
purely additive for clients that only know the wlr protocol.

## wlr-gamma-control (night light)

Night light clients do not draw a red overlay: they reprogram the display's gamma
lookup table through `wlr-gamma-control-unstable-v1`. Upstream advertises neither
that protocol nor `wp_color_manager_v1`, so the night light toggle in noctalia (or
`gammastep`, `wlsunset`, `wl-gammarelay`, …) silently does nothing — noctalia logs
`compositor does not support gamma control` and leaves the screen untouched.

| File | Change |
|---|---|
| `src/wayland/protocols/gamma_control.rs` | New. Manager global and per-output control. One control per output (a second one gets `failed`), `gamma_size` from the CRTC's `gamma_length`, the ramp read from the fd the client passes and handed to the backend. |
| `src/wayland/handlers/gamma_control.rs` | New. Implements the handler for `State` and delegates the two protocol objects. |
| `src/backend/kms/mod.rs` | `gamma_table_size` / `set_output_gamma`: find the CRTC(s) showing an output (mirrors included) and program the legacy ramp with `set_gamma`, retrying on `EBUSY`. |
| `src/wayland/handlers/output_power.rs` | Re-sends the last table when an output is powered back on, because a DPMS cycle drops the LUT. |

Details worth knowing when debugging it:

- The ramp goes through the legacy `SETGAMMA` ioctl, which the kernel turns into
  an atomic commit of the CRTC's `GAMMA_LUT` property. That is why it has to use
  the compositor's DRM master fd: a plain `open("/dev/dri/card0")` from another
  process fails with `EPERM`, so no helper daemon can do this behind the
  compositor's back.
- `crtc_info().gamma_length()` is 256 on Intel (the legacy palette size) while
  the atomic `GAMMA_LUT_SIZE` property is 1024. Clients are told 256 and the
  driver resamples, which is what the protocol expects.
- The last table is kept per output and re-sent on DPMS-on. A hotplug re-creates
  the surface with an identity ramp; a client that stays connected is expected to
  resend, as on any other compositor.
- Destroying the control — or the client disconnecting — restores the identity
  ramp, as the protocol requires.

## Install

```sh
# newest release
./scripts/install-latest.sh

# a specific upstream tag
./scripts/install-latest.sh epoch-1.11.0
```

Or by hand:

```sh
gh release download epoch-1.11.0 -R grigio/cosmic-comp-traffic \
   -p 'cosmic-comp-traffic-[0-9]*.pkg.tar.zst*' -D /tmp/cct
cd /tmp/cct && sha256sum -c ./*.pkg.tar.zst.sha256
sudo pacman -U ./*.pkg.tar.zst
```

The package is `cosmic-comp-traffic`; it `provides`/`conflicts`/`replaces`
`cosmic-comp` and installs the same binary to `/usr/bin/cosmic-comp`, so it is a
drop-in replacement and pacman removes the official package when you install it.

A new compositor only takes effect **at the next login**. To go back:

```sh
sudo pacman -S cosmic-comp
```

## How the automation works

[`.github/workflows/release.yml`](.github/workflows/release.yml):

1. **discover** — lists the tags of `pop-os/cosmic-comp`, keeps `epoch-X.Y.Z`
   with `X.Y.Z >= MIN_VERSION` (currently `1.10.0`), drops the ones that already
   have a release here. Runs every 6 hours, and on demand.
2. **build** — in an `archlinux:base-devel` container: installs the build deps,
   clones the upstream tag, applies `patches/common/*.patch` plus
   `patches/<version>/*.patch` if that directory exists, and runs `makepkg`
   (fat → thin LTO, `lld`), producing a signed-checksum `.pkg.tar.zst`.
3. **publish** — mirrors the upstream tag into this fork (so the release points
   at the upstream commit) and attaches the package plus its `.sha256` to a
   release named after the tag.
4. **prune** — deletes the packages of older `pkgrel`s from the same release,
   including their `.sha256` and `-debug` files, so a rebuilt tag carries
   exactly one build.

If a patch stops applying to a new upstream tag, the build fails and the release
is not created — the next scheduled run retries it, so a broken patch is visible
instead of silently shipping an unpatched compositor.

`pkgrel` is the number of commits that touched `PKGBUILD` or `patches/`, so any
patch change automatically produces a version pacman considers an upgrade. A
rebuild only keeps that newest `pkgrel`: older assets are pruned from the
release, because `gh release download -p '*.pkg.tar.zst*'` would otherwise fetch
several builds and `sudo pacman -U ./*` would install whichever one the shell
globs last.

### Triggers

| Event | What happens |
|---|---|
| `schedule` (every 6 h) | builds every missing tag ≥ `MIN_VERSION` |
| `workflow_dispatch` | builds one tag (`tag` input) or every missing one; `force` rebuilds existing releases |
| push to `master` touching `PKGBUILD`, `patches/**` or the workflow | rebuilds the newest tag and overwrites its release |

> Scheduled workflows are disabled by default in forks — keep them enabled under
> *Actions* in the repository settings. Upstream's own `ci.yml` and
> `require-checklist.yml` workflows are disabled here on purpose: they test
> upstream code, not the packaging.

The release asset is named `cosmic-comp-traffic-<version>-<pkgrel>-x86_64.pkg.tar.zst`
(the epoch is dropped from the file name because release assets cannot contain
`:`); the version inside the package is still `1:<version>-<pkgrel>`, matching
the official `cosmic-comp` epoch.

## Adding a patch for a new upstream release

Patches are applied at build time, so nothing has to be rebased:

1. If the change is generic, update `patches/common/*.patch` (regenerate against
   the newest tag).
2. If a new upstream tag needs an extra fix, add
   `patches/<version>/0001-something.patch` — it is applied after `common/`
   for that version only.
3. Push. The workflow rebuilds the newest tag; `pkgrel` goes up by one.

Keep `MIN_VERSION` in the workflow in sync if you ever want older tags built.

## Build locally

```sh
./scripts/build-local.sh            # newest upstream tag → ./out/
./scripts/build-local.sh 1.10.0     # specific version
```

It copies `PKGBUILD` and `patches/` to a scratch dir (makepkg's `src/` would
otherwise collide with the cosmic-comp source tree) and calls `makepkg` with
`COSMIC_COMP_VERSION`/`COSMIC_COMP_PKGREL` set, the same way CI does.

---

# cosmic-comp (upstream)
Compositor for the COSMIC desktop environment

## Development

This project uses [just](https://github.com/casey/just) as a command runner.

### Available Commands

#### Building
- `just build-debug` - Compile with debug profile
- `just build-release` - Compile with release profile (default)
- `just build-vendored` - Compile release profile with vendored dependencies
    - Requires vendoring first, which can be done with `just vendor`

#### Code Quality
- `just check` - Run clippy linter with pedantic warnings

#### Installation
- `just install` - Install binary and configuration files
- `just install-bare-session` - Install with bare session files (wayland session, systemd units)
- `just uninstall` - Remove installed files
- `just uninstall-bare-session` - Remove installed files including bare session files

#### Cleanup
- `just clean` - Run `cargo clean`
- `just clean-dist` - Run `cargo clean` and remove vendored dependencies

#### Vendoring
- `just vendor` - Vendor dependencies locally and create vendor.tar
- `just vendor-extract` - Extract vendored dependencies from vendor.tar
