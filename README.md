# cosmic-comp-traffic

Fork of [pop-os/cosmic-comp](https://github.com/pop-os/cosmic-comp) that adds
macOS-style traffic-light window decorations and a soft focus ring, implements
`zwlr_foreign_toplevel_manager_v1` so third-party docks and taskbars work, and
ships a ready-to-install **Arch Linux package for every upstream release tag**.

- Releases: <https://github.com/grigio/cosmic-comp-traffic/releases>
- Base version: upstream tag `epoch-1.10.0` and newer

## What the patches change

| File | Change |
|---|---|
| `src/shell/element/window.rs` | `DefaultDecorations::view` no longer uses libcosmic's `header_bar()`; the titlebar is a themed container with a canvas-drawn traffic light per control (close `#FF5F57`, minimize `#28C840`, maximize `#FEBC2E`, grey `#636366` when unfocused; glyph appears on hover). `SSD_HEIGHT` 36 → 28 so hit-testing matches the new header. `push_render_elements` skips the grey `bg_divider` frame on the focused window, because the focus ring is the outline then. |
| `src/shell/element/window.rs` | `set_activate` stores the `activated` flag *before* queueing the focus event and uses `force_update()` instead of `force_redraw()`, so the traffic lights follow focus immediately. |
| `src/backend/render/shaders/rounded_outline.frag` | Signed-distance ring with `FEATHER = 1.5` logical px of falloff on both edges instead of a hard band. |
| `src/backend/render/mod.rs` | Focus ring element gets 2 px of extra geometry (`pad`) so the glow is not clipped. Other indicators keep `pad: 0`. |

Both patches live in `patches/common/` and apply cleanly to `epoch-1.10.0` and
`epoch-1.11.0`:

- [`0001-macos-traffic-light-decorations.patch`](patches/common/0001-macos-traffic-light-decorations.patch)
  — the traffic lights and the focus ring described above.
- [`0002-wlr-foreign-toplevel-management.patch`](patches/common/0002-wlr-foreign-toplevel-management.patch)
  — `zwlr_foreign_toplevel_manager_v1`, described below.

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
   -p '*.pkg.tar.zst*' -D /tmp/cct
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

If a patch stops applying to a new upstream tag, the build fails and the release
is not created — the next scheduled run retries it, so a broken patch is visible
instead of silently shipping an unpatched compositor.

`pkgrel` is the number of commits that touched `PKGBUILD` or `patches/`, so any
patch change automatically produces a version pacman considers an upgrade.

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
