<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# CODE-SIGNING.md: the two kinds of signing, and why binaries need one too

Two different things share the word "signing" in this repository, and
confusing them costs a day.

| | Signs | Purpose | Set up with |
| --- | --- | --- | --- |
| **Commit signing** | Git commits and tags | Provenance of the source; `main` requires it | `just setup-signing`, checked by `just doctor-signing` |
| **Code signing** | The built binaries | macOS keeps recognising the program across rebuilds; releases pass Gatekeeper | `just setup-codesign`, checked by `just doctor-codesign` |

This page is for whoever signs md2html's commits or its binaries. The first
is mandatory for everyone (AGENTS.md sections 6 and 10; the gate
`commit-signatures`). The second matters the moment a program asks macOS
for anything privacy-sensitive (microphone, input monitoring, Full Disk
Access, Photos, Mail) or ships to people who did not build it; md2html asks
for nothing of the kind, so for it the second matters when it ships. Everything here lives in `just/codesign.just`,
`packaging/macos/md2html.entitlements` and the macOS steps of `release.yml`.
Nothing in it uses a key of the template's authors: you bring your own
identity, or the recipe makes a local one.

## 1. Why the binary must be signed with a stable identity

macOS privacy (TCC) keys a permission grant to the binary's **Designated
Requirement** (DR). For an unsigned or ad-hoc-signed binary the DR is the
binary's code hash, so **every `cargo build` is a new application in TCC's
eyes and yesterday's grant silently stops applying.** The failure is nasty
because it is silent: the feature simply stops working, nothing logs an
error, and the afternoon goes to debugging the wrong layer.

Signing with a certificate identity changes the DR from a hash to
`identifier "<bundle id>" and certificate ...`, which survives rebuilds. A
**self-signed certificate in the login keychain is enough** for
development: it is stable per machine and TCC accepts it. Nothing is
distributed and no Apple developer account is involved.

`just doctor-codesign` prints the verdict per built binary: a DR containing
`cdhash` means the grant dies on the next rebuild; one containing
`certificate` is what you want.

## 2. Setup, once per machine

```bash
just setup-codesign        # non-interactive; creates the self-signed `md2html-dev` identity
just doctor-codesign       # identity present? built binaries certificate-pinned?
```

The recipe generates the certificate with `openssl` and imports it with
`security import`. Three things in it were established by testing rather
than by reading documentation, and are worth knowing before touching it:

- the certificate needs the code-signing extended key usage set explicitly;
- the PKCS#12 needs a real password, or `security import` fails with a
  misleading "MAC verification failed" error;
- OpenSSL 3 needs `-legacy` to write a PKCS#12 macOS can read, while
  LibreSSL (the stock `/usr/bin/openssl`) rejects `-legacy` and is already
  compatible, hence the try-then-fall-back.

No trust setting is needed: `codesign` signs happily with an untrusted
self-signed identity, so the recipe never raises an authorization dialog.
`set-key-partition-list` after the import is what stops the first
`codesign` call from raising a keychain dialog per binary.

## 3. The development loop

```bash
just build-signed              # cargo build + codesign, in that order (debug)
just build-signed release      # same for target/release
just sign-binaries release     # sign what is already built
just doctor-codesign           # the DR check
just resign-installed ~/bin    # re-sign installed copies without killing running processes
```

Use `just build-signed` rather than a bare `cargo build` while working on
anything that touches a privacy grant. Cargo has no post-build hook
(`build.rs` runs before the binary exists), so `just` is the seam: every
recipe that produces binaries can call `just sign-binaries` at its tail.

**Identity preference**, resolved by the recipes in this order:

1. `MD2HTML_CODESIGN_IDENTITY`, when set: any codesigning identity name in
   the keychain, e.g. `Developer ID Application: Your Org (TEAMID)`.
2. A `Developer ID Application:` identity in the keychain, when exactly one
   exists (several: set the variable). Developer ID signing applies the
   hardened runtime and `packaging/macos/md2html.entitlements`.
3. The self-signed `md2html-dev` from `just setup-codesign`.

`MD2HTML_CODESIGN_IDENTIFIER` is the identifier baked into every signature;
TCC keys on it together with the certificate, so keep it stable and unique
to the project. The default is `org.githubrobbi.md2html` (`just/codesign.just`),
in the reverse-DNS shape bundle identifiers and launchd labels use. Set the
variable (for example in `.cargo/config.toml` under `[env]`) when the
program already has a bundle identifier elsewhere; the two must agree.

**Entitlements** (`packaging/macos/md2html.entitlements`) are empty by
design: a plain command-line tool needs nothing under the hardened runtime.
Add only what the program does, each with its reason: `device.audio-input`
for a microphone, `cs.disable-library-validation` only when the program
dlopens a library signed by another team. Restricted entitlements
(application identifier, keychain access groups, Secure Enclave) are
honoured only inside an `.app` bundle with an embedded provisioning
profile; a bare binary carrying one is killed at launch.

## 4. Granting permissions, and why the order matters

md2html asks macOS for no permission and runs no daemon; this section is
background for a program that does.
Grants attach to the **responsible process**. Started from a terminal,
macOS attributes the request to the terminal and reuses its grants, which
is why a hand-run tool "works" and a `launchd`-started daemon with the same
binary does not. So, for a program that runs unattended:

1. a present human runs the program once from a terminal (a `doctor`
   subcommand that touches the resource is the natural place), so the
   prompts arrive while someone can answer them;
2. the grant is attributed to the signed identity, not to the terminal;
3. the LaunchAgent starts the daemon later and the grant is already there.

A grant takes effect only after the process is relaunched, and the system
APIs that report a grant can disagree with whether the resource actually
works; report both when you build a doctor.

## 5. Releases: Developer ID and notarization (optional, secrets-gated)

`release.yml` re-signs every macOS binary after `strip` (stripping
invalidates the linker's ad-hoc signature; macOS 26+ kills such a binary at
exec). It signs one of two ways:

- **Developer ID, hardened runtime, Apple timestamp, then notarization**
  when these repository secrets exist. Users get no Gatekeeper prompt.
- **ad-hoc** otherwise: the binary runs, and Gatekeeper shows the standard
  "open anyway" prompt on first launch. This is the default for a fresh
  project and for forks.

| Secret | What |
| --- | --- |
| `APPLE_SIGNING_IDENTITY` | the full identity string, `Developer ID Application: Your Org (TEAMID)` |
| `APPLE_DEVELOPER_ID_P12` | the signing identity with its private key, exported from the keychain, base64-encoded |
| `APPLE_DEVELOPER_ID_P12_PASSWORD` | the password that opens the `.p12` |
| `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`, `APPLE_API_KEY_P8` | an App Store Connect API key (Developer role) for `notarytool`; the `.p8` base64-encoded. Without these three the build is signed but not notarized, with a warning |

`release.yml` reads these six. `just secrets-sync-apple [gopass-prefix]`
pushes them, and the team id as `APPLE_TEAM_ID` (which no workflow reads
today), from a
`gopass` store (`keys/apple/team-id`, `.../signing-identity`,
`.../developer-id-application.p12`, `.../developer-id-p12-password`,
`.../asc-api-key-id`, `.../asc-api-issuer-id`, `.../asc-api-key.p8`) in one
step; set them by hand with `gh secret set` if you keep secrets elsewhere.

Practical notes from setting this up: a Developer ID certificate
is 2048-bit RSA (the only key type Apple issues it under); the `.p12` macOS
exports uses a legacy RC2 wrapping that OpenSSL 3 refuses without
`-legacy`, so it is imported with `security import`, never OpenSSL; the
App Store Connect `.p8` can be downloaded exactly once. The notarization
ticket is checked online for bare binaries; a stapled ticket (offline
launch) needs a `.pkg`, `.dmg` or `.app`. Importing the same `.p12` into a
developer's login keychain makes local builds carry the release signature;
the TCC grants then re-prompt once, because they were keyed on the previous
identity.

## 6. What this repository does not do

- It does not produce an `.app` bundle, `.pkg` or `.dmg`; those are
  product decisions (a bundle is also the only way to carry restricted
  entitlements). No recipe builds one; a bundle would start as a new
  recipe beside `just/codesign.just`.
- It does not sign Windows binaries (Authenticode) or Linux packages.
  Nor does any binary embed Windows resources (icon, version block);
  that would be a `build.rs` addition (for example with `winresource`).
