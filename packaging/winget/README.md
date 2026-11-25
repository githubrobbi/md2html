<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# WinGet: `SkyLLC.md2html`, and its first submission

md2html is published on the Windows Package Manager community repository,
microsoft/winget-pkgs, as **`SkyLLC.md2html`**, beside the same publisher's
`SkyLLC.UFFS`, and in the same way: a **zip → portable** installer (the
release's `md2html-windows-x64.zip`) whose one executable,
`md2html-windows-x64\md2html.exe`, is put on `PATH` as the command
`md2html` (`NestedInstallerFiles` / `PortableCommandAlias`). The Windows
binary is not signed (decided 2026-11-24).

Every version after the first is submitted by
[`winget-publish.yml`](../../.github/workflows/winget-publish.yml) through
`winget-releaser` (komac), from the fork `githubrobbi/winget-pkgs`, which it
fast-forwards to upstream first. komac **updates** a package from the
manifest of its previous version, so it cannot make the first one: that is
submitted by hand, once, with the steps below. Keep `LANE_WINGET` off until
that pull request has merged, or the release's own submission fails.

## The manifests

The three templates here are the package's manifests, schema 1.28 as
`SkyLLC.UFFS` uses:

| Template | Manifest |
| --- | --- |
| [`SkyLLC.md2html.yaml.in`](SkyLLC.md2html.yaml.in) | the version manifest |
| [`SkyLLC.md2html.installer.yaml.in`](SkyLLC.md2html.installer.yaml.in) | the installer: zip, nested portable, the alias, the archive's URL and SHA-256, the release date |
| [`SkyLLC.md2html.locale.en-US.yaml.in`](SkyLLC.md2html.locale.en-US.yaml.in) | the default locale: publisher `Sky, LLC` (as `SkyLLC.UFFS` names it), the licence `(MIT OR Apache-2.0) AND BSD-3-Clause` (md2html's own and the WHATWG's notice for the entity table, `NOTICE`), the description |

They are not filled by hand. `just winget-manifests vX.Y.Z`
([`scripts/release/winget_manifests.sh`](../../scripts/release/winget_manifests.sh))
writes them for the published release: the version from the tag, the
SHA-256 from the release's `SHA256SUMS`, checked against the downloaded
archive (which must hold `md2html-windows-x64/md2html.exe`), and the date
it was published. It refuses a draft: with `lane:codesign`, run it after
`just release-sign-macos` has published the release. Its test is
`scripts/release/test_winget_manifests.sh` (gate `gate-tests`).

## The first submission (0.8.0), step by step

This is how `SkyLLC.UFFS` was first submitted
([microsoft/winget-pkgs#378294](https://github.com/microsoft/winget-pkgs/pull/378294),
merged 2026-05-29): one pull request from the fork, one commit adding the
three files under `manifests/s/SkyLLC/<Package>/<version>/`. The owner
runs it, after 0.8.0 is published; the Contributor License Agreement is
already signed for the account.

1. **The manifests.** In this repository, on an up-to-date `main`:

   ```bash
   just winget-manifests v0.8.0
   ls target/winget/manifests/s/SkyLLC/md2html/0.8.0/
   # SkyLLC.md2html.installer.yaml  SkyLLC.md2html.locale.en-US.yaml  SkyLLC.md2html.yaml
   ```

   Read the three files. On a Windows machine,
   `winget validate --manifest <that folder>` and
   `winget install --manifest <that folder>` check them before the pull
   request; without one, the pull request's own validation does (UFFS's was
   submitted that way, from a Mac).

2. **The fork, level with upstream** (komac and the pull request both
   start from it; a token without the `workflow` scope is refused when
   upstream changed a workflow file, and `gh`'s own token has it):

   ```bash
   gh api -X POST repos/githubrobbi/winget-pkgs/merge-upstream -f branch=master
   ```

3. **A branch with the three files**, in a sparse clone of the fork
   (winget-pkgs is large):

   ```bash
   work="$(mktemp -d)"
   git clone --depth 1 --filter=blob:none --sparse https://github.com/githubrobbi/winget-pkgs.git "$work/winget-pkgs"
   cd "$work/winget-pkgs"
   git sparse-checkout set manifests/s/SkyLLC
   git switch -c add-skyllc-md2html-0.8.0
   mkdir -p manifests/s/SkyLLC/md2html
   cp -R <md2html checkout>/target/winget/manifests/s/SkyLLC/md2html/0.8.0 manifests/s/SkyLLC/md2html/
   git add manifests/s/SkyLLC/md2html/0.8.0
   git commit -m "New package: SkyLLC.md2html version 0.8.0"
   git push -u origin add-skyllc-md2html-0.8.0
   ```

4. **The pull request**, its title as winget-pkgs names a new package, its
   body the repository's current template filled in (UFFS's, adapted,
   follows):

   ```bash
   gh pr create --repo microsoft/winget-pkgs --base master \
     --head githubrobbi:add-skyllc-md2html-0.8.0 \
     --title "New package: SkyLLC.md2html version 0.8.0" --body-file body.md
   ```

   ```markdown
   ## 📖 Description

   New package submission: **SkyLLC.md2html** (md2html) v0.8.0.

   md2html turns Markdown into one self-contained HTML page with its own outline,
   search, themes, glossary, footnotes, coloured code, maths and diagrams, in
   seventeen languages, and also writes EPUB 3 and PDF. The Windows zip ships one
   portable executable, `md2html.exe`, aliased on PATH via `NestedInstallerFiles`.

   Upstream release: https://github.com/githubrobbi/md2html/releases/tag/v0.8.0

   ## ✅ Checklist

   - [x] Signed the [Contributor License Agreement](https://cla.opensource.microsoft.com)
   - [ ] Linked to an issue (if applicable) — N/A, new package

   ## 📦 Manifest Checklist

   - [x] Checked that there aren't other open pull requests for the same manifest update/change
   - [x] This PR only modifies one (1) manifest (new package, 3 files in `manifests/s/SkyLLC/md2html/0.8.0/`)
   - [ ] Validated manifest locally with `winget validate --manifest <path>` — author is on macOS, where `winget` is unavailable; relying on automated validation in this repo
   - [ ] Tested manifest locally with `winget install --manifest <path>` — same reason; will validate via wingetbot pipeline
   - [x] Manifest conforms to the [1.28 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.28.0)
   ```

5. **Validation.** wingetbot's pipeline installs the package in a clean
   Windows and labels the pull request; a moderator merges a new package.
   An unsigned Rust binary can trip Windows Defender's heuristics
   (`Validation-Defender-Error`), as `SkyLLC.UFFS` did several times: the
   bytes match the release's `SHA256SUMS`; submit the archive's binary as a
   false positive at <https://www.microsoft.com/en-us/wdsi/filesubmission>
   (UFFS's `packaging/winget/README.md` has the drill) and ask on the pull
   request for a new validation once the analyst has cleared it.

6. **After the merge.** `winget install --id SkyLLC.md2html`, then
   `md2html --version` prints `md2html 0.8.0`. Then the owner sets
   `WINGET_TOKEN` (a classic token of `githubrobbi`, scopes `public_repo`
   and `workflow`, about a year), and `gh variable set LANE_WINGET --body
   true`; dispatch `winget-token-expiry-check.yml` once to see the token
   and the fork ready. From the next release on, `winget-publish.yml`
   submits each version by itself.

## Later versions

Nothing to do: komac copies the previous manifest and changes the version,
the URL, the SHA-256 and the date, keeping the nested file and its alias.
A second executable bundled into the zip would need its
`NestedInstallerFiles` entry added to the manifest once (UFFS seeds its
extra aliases automatically from a list); md2html has one.
