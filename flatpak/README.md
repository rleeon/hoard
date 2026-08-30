# Hoard as a Flatpak

Two things live here: a manifest that builds Hoard locally, and everything
Flathub would need that a local build doesn't.

## Building it here

```sh
scripts/build-flatpak.sh            # build + install --user
scripts/build-flatpak.sh --bundle   # also write flatpak/hoard.flatpak
flatpak run services.hoard.saves
```

It follows the binary recipe from the Tauri docs: build the app's own `.deb`
the way everything else here builds it, then unpack that into `/app` rather
than rebuild Rust and the frontend inside the sandbox. That keeps one build
path instead of two that drift.

`shared-modules/` is vendored verbatim from
[flathub/shared-modules](https://github.com/flathub/shared-modules) and carries
libappindicator, which the tray needs and the GNOME runtime doesn't ship.

## What the sandbox changes

**The app doesn't update itself.** `/app` is read-only and the next version
comes from whichever remote it was installed from, so the updater reports the
install as managed and stops there. That is deliberate — see
`Delivery::Managed` in `crates/hoard-agent/src/install/mod.rs`.

**Login start goes through a portal.** There is no `systemctl` in the runtime
and `$XDG_CONFIG_HOME` points inside the sandbox, so the usual systemd user
unit would be written where nothing reads it. `hoardd` asks
`org.freedesktop.portal.Background` instead, and the portal writes the host
entry that starts it again at the next login. The first request may put a
dialog in front of the user; refusing it is reported rather than swallowed.

**`--filesystem=host` is not negotiable here.** Hoard's whole job is finding
saves wherever they live — Steam libraries on other drives, Proton prefixes,
emulator folders, paths the user picks — so a handful of narrower grants would
silently miss whichever drive a game happens to be on. Expect a reviewer to
ask; that is the answer.

**The CLI is inside the sandbox.** `hoard` ships in `/app/bin` next to the app,
which means it is not on the host's `PATH`. Run it through Flatpak:

```sh
flatpak run --command=hoard services.hoard.saves status
```

Worth an alias if you use it often:

```sh
alias hoard='flatpak run --command=hoard services.hoard.saves'
```

Anything driving Hoard from outside the sandbox — scripts, agents, `--json`
consumers — needs that prefix too, or the native package instead.

## What submitting to Flathub still needs

1. **A manifest whose sources are remote.** Flathub builds nothing of ours: it
   clones `flathub/services.hoard.saves` and runs flatpak-builder there, so the
   local `path: hoard.deb` can't survive the trip. Generate the submittable one
   from a released tag:

   ```sh
   scripts/flathub-manifest.sh v1.1.5
   ```

   It reads the manifest here, swaps that one source for the release asset and
   the `sha256` published beside it, and writes `flatpak/flathub/`. The
   metainfo and `shared-modules/` go in that repo unchanged.

   Reviewers may still ask for a build from source, since Hoard is AGPL. That
   is a different and much larger job — vendoring every cargo and npm
   dependency as manifest sources — and it is a decision, not an oversight.

2. **Screenshots.** At least one, in English, hosted somewhere stable, PNG or
   JPEG. See the comment in the metainfo for why the ones on hoard.services
   don't qualify.

3. **A release entry per version**, in the metainfo, using the published
   counter and not the workspace one.

4. **Domain verification** for `services.hoard.saves`, which is `hoard.services`
   read backwards and therefore ours to prove.

Nothing here is wired into CI. Merging it publishes nothing; the app on
Flathub only exists once someone opens that pull request.
