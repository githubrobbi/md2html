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
`packaging/macos/md2html.entitlements`, `scripts/release/` and the macOS steps of `release.yml`.
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

## 5. Releases: Developer ID and notarization, on the owner's Mac

Decided 2026-11-24: **no Apple key or credential ever goes to GitHub.** A
key kept as an Actions secret is readable by every workflow that runs with
it, and by whoever can push a workflow or holds the owner's token; once
copied it signs as Sky, LLC until Apple revokes the certificate, and the
revocation stops every binary that certificate ever signed. So the macOS
binaries are signed and notarized on the owner's Mac, from the keychain,
in a second stage of the release. Nothing in `release.yml` reads an
`APPLE_*` secret, and there are none to set.

### The two stages

| Stage | Where, by whom | What it does | What it holds |
| --- | --- | --- | --- |
| 1. `release.yml` | GitHub's runners, started by the push of the signed tag | Builds the four targets from the tag's commit, signs the macOS binaries **ad-hoc** (stripping breaks the linker's signature, and macOS 26+ kills such a binary), packs the archives with `scripts/release/pack_zip.sh`, writes the SBOMs, `CHECKSUMS.txt` and `SHA256SUMS` (`scripts/release/checksums.sh`), attests every asset (`lane:slsa`), and creates the release | No Apple secret; the workflow token |
| 2. `just release-sign-macos vX.Y.Z` | The owner's Mac, by the owner | Verifies the draft, signs and notarizes the two macOS binaries, packs their archives again, writes the two lists again, replaces those six assets, publishes the release, dispatches Homebrew, winget and crates.io where their lanes are on (`scripts/release/sign_macos.sh`) | The Developer ID identity in the login keychain (a YubiKey later); the notary key in a keychain profile |

Which of the two a release takes is the repository variable
`LANE_CODESIGN` (`docs/forge/COMPONENTS.md`, `lane:codesign`):

- **off** (unset or `false`): `release.yml` publishes the release with
  ad-hoc signed macOS binaries and dispatches the package managers. Users
  see Gatekeeper's "open anyway" prompt on the first launch of a
  downloaded copy (not for `brew install` or `curl`, which set no
  quarantine flag).
- **`true`**: `release.yml` creates the release as a **draft** with every
  asset and dispatches nothing; it waits for stage 2. It needs `lane:slsa`
  too, since stage 2 refuses assets without provenance.

What GitHub never sees: the certificate's private key, any `.p12` or its
password, the App Store Connect `.p8`, its key ID and issuer. It sees the
signed binaries, which are public anyway.

### What `just release-sign-macos` checks, and in which order

It refuses before downloading anything unless: it runs on macOS; the tag
is `vX.Y.Z`; `LANE_RELEASE`, `LANE_CODESIGN` and `LANE_SLSA` are on; the
tag is annotated, its signature verified by GitHub, and it names a commit
(as `release.yml` checks it); its release exists and is a draft; exactly
one Developer ID Application identity qualifies; and the notary profile
signs in. The identity is the one gopass names in `keys/apple/signing-identity`
(its name, or its SHA-1 when two certificates share the name, as the
dedicated certificate will with the older one), else the keychain's only
`Developer ID Application:` identity; it signs by the identity's SHA-1.

Then, before signing: every asset of the draft is listed in
`CHECKSUMS.txt` with its SHA-256 and the list is what `checksums.sh`
writes for those files; every asset verifies with `gh attestation verify
--repo githubrobbi/md2html --signer-workflow
githubrobbi/md2html/.github/workflows/release.yml`, from the push of the
tag at its commit (`--source-ref refs/tags/vX.Y.Z --source-digest
<commit>`), or, for a run started again by dispatch, from `main`; each
macOS archive holds one flat folder with the binary and `NOTICE`, its
binary is the raw asset, and `pack_zip.sh` on this Mac packs exactly the
archive `release.yml` packed from the same files.

Then: `codesign --force --options runtime --timestamp --identifier
org.githubrobbi.md2html --entitlements <the file at the tag's commit>
--sign <SHA-1>`; `codesign --verify --strict` and `codesign -dvv` (the
authority, the identifier, the time stamp, the hardened runtime); one ZIP
of both binaries (`ditto -c -k --keepParent`, as Apple wants a bare binary
sent) to `xcrun notarytool submit --wait --keychain-profile
md2html-notary`, its log printed when Apple does not answer `Accepted`;
`spctl --assess --type open --context context:primary-signature`, which
must say `source=Notarized Developer ID`; the two archives packed again
around the signed binaries, the two lists written again, and a check
that exactly the four macOS lines changed; `gh release upload --clobber`
of the six files, read back and compared; `gh release edit
--draft=false`; the dispatches of `brew-publish.yml`, `winget-publish.yml`
and `release-plz.yml` that `release.yml` would have made (each only when
its lane is on).

Nothing is published until all of that has passed. A failure leaves the
release a draft and says so; `just release-sign-macos vX.Y.Z` again starts
over from the verified downloads kept in `target/release-sign/vX.Y.Z/`,
and accepts on the draft only those files or ones it uploaded itself (by
SHA-256), so it can be run again after a refused notarization or a
broken upload. A draft that is neither (changed by hand, or first touched
from another machine) is refused: delete it (`gh release delete vX.Y.Z
--yes`, the tag stays) and dispatch `release.yml` for the tag again. Once
published, a release is never changed; a dispatch that failed after
publishing is printed to run by hand.

`scripts/release/test_sign_macos.sh` (gate `gate-tests`) runs all of this
against stand-ins for `gh`, `codesign`, `xcrun`, `security`, `spctl`,
`ditto` and `gopass`: each refusal, a refused notarization and a failed
publish followed by a second run, and a run that replaces exactly the six
files.

### Provenance: built by the workflow, then signed by the owner

The SLSA attestation `release.yml` makes covers the macOS binaries and
archives **as built**, ad-hoc signed, and the `CHECKSUMS.txt` that listed
them. Stage 2 verifies exactly those before it signs. The signed files
then differ from them (a signature is bytes in the file), so the published
macOS binaries, their archives, `CHECKSUMS.txt` and `SHA256SUMS` no longer
match the attestation; every other asset still does. The chain is: built
by `release.yml` from the tag's commit (the attestation, checked by the
owner before signing), then signed by Sky, LLC's Developer ID and
notarized by Apple (`codesign --verify --strict md2html`, `spctl --assess
--type open --context context:primary-signature md2html`). The release
notes say the same.

### Setup, once (the owner)

1. **A Developer ID Application certificate for md2html releases only.**
   The Account Holder creates it (Apple allows five) at
   developer.apple.com, Certificates, from a new certificate signing
   request made in Keychain Access on this Mac, so the private key is made
   in the login keychain and never leaves it (later: made on a YubiKey,
   PIV slot 9c, which `codesign` uses through macOS's own driver). Install
   the downloaded `.cer`; `security find-identity -v -p codesigning` lists
   it. Put its SHA-1 into gopass, since it shares its name with the older
   certificate: `gopass insert keys/apple/signing-identity`. A revocation
   of this certificate then stops md2html's macOS releases and nothing
   else Sky, LLC signs. If a backup is wanted, a `.p12` of this one
   identity goes into gopass, encrypted to the owner's keys only; never to
   GitHub.

   Until then (the owner's choice, 2026-11-26), releases are signed with
   the Developer ID Application certificate already in the login
   keychain, `Sky, LLC. (RD3S54N68V)`, which docenta's releases use too:
   it is the keychain's only Developer ID identity, and gopass
   `keys/apple/signing-identity` names it. Its revocation stops the
   releases of both.
2. **An App Store Connect team API key with the Developer role**, named
   for md2html notarization, made by the Account Holder or an Admin (Users
   and Access, Integrations). The Developer role is the least that
   `notarytool` is reported to accept; confirm it with the first
   submission. Download the `.p8` (possible once) into gopass, then store
   it in the keychain once, from a file that lives only for the command:

   ```bash
   key="$(mktemp -d)"; gopass cat keys/apple/asc-api-key.p8 > "$key/AuthKey.p8"
   xcrun notarytool store-credentials md2html-notary \
     --key "$key/AuthKey.p8" --key-id <key ID> --issuer <issuer ID>
   rm -rf "$key"
   xcrun notarytool history --keychain-profile md2html-notary   # signs in
   ```

3. `gh variable set LANE_CODESIGN --body true` (with `LANE_SLSA`), before
   the tag of the first release that is to be signed.
4. After each release, `xcrun notarytool history --keychain-profile
   md2html-notary` lists every submission of the team: one you did not make
   means the key is in other hands.

### Revocation, when the key may be in other hands

The Mac lost or compromised, a submission in `notarytool history` that
is not yours, a signed binary you did not make:

1. Revoke the API key in App Store Connect (Users and Access,
   Integrations, the key, Revoke). It takes effect at once and cannot be
   undone; make a new one when needed (setup, step 2).
2. Email product-security@apple.com, from the Account Holder, asking to
   revoke the certificate: the team ID, the certificate's name and serial
   number (of the certificate whose SHA-1 is in gopass: Keychain Access
   shows both in its details; `security find-certificate -a -Z -p -c
   "Developer ID Application"` prints every such certificate with its
   SHA-1, and `openssl x509 -noout -serial -subject` reads a PEM of
   them), and why. Developer ID certificates are not revoked from the
   website. After revocation, Apple says, a binary signed with that
   certificate can no longer be installed, nor launched if it is installed
   already: every macOS binary of every md2html release it signed, and
   (being dedicated) nothing else.
3. A new certificate (setup, step 1), then a new release, the next patch,
   signed with it; its notes say why. The revoked releases' assets cannot
   be replaced (published releases are immutable).
4. Read the GitHub account's security log and the repository's Actions
   history as well: the release path starts from the owner's signed tag.

The certificate is valid for five years; signatures made with Apple's time
stamp stay valid after it expires, so renewal (a new certificate before
the old one runs out, `docs/MAINTAINING.md`) matters for new releases only.

### Practical notes

A Developer ID certificate is 2048-bit RSA (the only key type Apple
issues it under). A `.p12` macOS exports uses a legacy RC2 wrapping that
OpenSSL 3 refuses without `-legacy`, so it is imported with `security
import`, never OpenSSL. The notarization ticket is checked online for bare
binaries; a stapled ticket (offline launch) needs a `.pkg`, `.dmg` or
`.app`. Having the release identity in the login keychain makes `just
build-signed` sign local builds with it too (identity preference, section
3); the TCC grants then re-prompt once, because they were keyed on the
previous identity.

## 6. What this repository does not do

- It does not produce an `.app` bundle, `.pkg` or `.dmg`; those are
  product decisions (a bundle is also the only way to carry restricted
  entitlements). No recipe builds one; a bundle would start as a new
  recipe beside `just/codesign.just`.
- It does not sign Windows binaries (Authenticode) or Linux packages.
  Nor does any binary embed Windows resources (icon, version block);
  that would be a `build.rs` addition (for example with `winresource`).
