# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC

# ─────────────────────────────────────────────────────────────────────
# BINARY code signing (macOS). `just setup-signing` signs git COMMITS;
# this file signs the BINARIES you build, so macOS keeps recognising
# them across rebuilds. Different subject, deliberately parallel names.
#
# Why it matters: macOS keys what it remembers about a binary - a
# privacy (TCC) grant such as Full Disk Access or Documents access, the
# application firewall's "accept incoming connections" answer for
# `md2html --serve` - to the binary's Designated Requirement. For an
# unsigned or ad-hoc-signed binary the DR is its code hash, so EVERY
# `cargo build` is a new application in macOS's eyes and yesterday's
# answer stops applying. A certificate identity makes the DR "identifier +
# certificate", which survives rebuilds. A self-signed certificate in
# the login keychain is enough for development; nothing is
# distributed and no Apple developer account is involved.
#
# Identity preference (docs/forge/CODE-SIGNING.md):
#   1. MD2HTML_CODESIGN_IDENTITY, when set (any codesigning identity name
#      in the keychain, e.g. "Developer ID Application: Your Org (TEAMID)")
#   2. a "Developer ID Application:" identity in the keychain, when
#      exactly one exists (also notarisable; hardened runtime and
#      packaging/macos/md2html.entitlements are applied)
#   3. the self-signed `md2html-dev` from `just setup-codesign`
#
# Setup once:  just setup-codesign
# Build+sign:  just build-signed [debug|release]
# Check:       just doctor-codesign
# ─────────────────────────────────────────────────────────────────────

# The identifier baked into every signature: TCC and the DR key on it,
# so it must be stable across builds and unique to this project.
codesign_identifier := env("MD2HTML_CODESIGN_IDENTIFIER", "org.githubrobbi.md2html")

# Resolve the signing identity by the preference ladder; prints the
# identity name, or nothing when none qualifies (an answer the callers
# act on). A keychain that cannot be read is an error. Internal helper.

# Print the signing identity the preference ladder resolves to (internal).
_codesign-identity:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "$(uname -s)" == "Darwin" ]] || exit 0
    if [[ -n "${MD2HTML_CODESIGN_IDENTITY:-}" ]]; then
        printf '%s\n' "$MD2HTML_CODESIGN_IDENTITY"; exit 0
    fi
    identities=$(security find-identity -v -p codesigning)
    # awk prints the matches (or nothing) and exits 0 either way: "no
    # Developer ID" is an answer here, not a failure.
    dev_ids=$(awk -F'"' '/"Developer ID Application: [^"]*"/ { print $2 }' <<< "$identities" | sort -u)
    count=$(awk 'NF' <<< "$dev_ids" | wc -l | tr -d ' ')
    if [[ "$count" -eq 1 ]]; then
        printf '%s\n' "$dev_ids"; exit 0
    fi
    if [[ "$count" -gt 1 ]]; then
        printf "  \033[1;33m⚠  %s Developer ID identities in the keychain - set MD2HTML_CODESIGN_IDENTITY to pick one:\033[0m\n" "$count" >&2
        printf '%s\n' "$dev_ids" | sed 's/^/     /' >&2
    fi
    if [[ "$identities" == *"md2html-dev"* ]]; then
        printf 'md2html-dev\n'; exit 0
    fi
    exit 0

# Sign every Mach-O executable in target/<profile> with the best
# available identity (macOS only; a silent no-op everywhere else).
#
# Cargo has no post-build hook (build.rs runs BEFORE the binary exists),
# so `just` is the seam: any recipe that produces binaries calls this at
# its tail, and binaries are signed whether they are installed or run
# straight out of target/. A missing identity is an error: run
# `just setup-codesign` once.

# Sign every Mach-O executable in target/<profile> with the best identity (macOS).
sign-binaries profile="release":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "$(uname -s)" != "Darwin" ]]; then
        printf "  \033[0;36mℹ️  not macOS - binary signing is a macOS concern; nothing signed\033[0m\n"
        exit 0
    fi
    identity="$(just _codesign-identity)"
    if [[ -z "$identity" ]]; then
        printf "  \033[0;31m❌ no code-signing identity in the keychain (Developer ID or md2html-dev)\033[0m\n"
        printf "     Fix: just setup-codesign\n"
        exit 1
    fi
    extra=(--identifier "{{ codesign_identifier }}")
    if [[ "$identity" == "Developer ID Application:"* ]]; then
        extra+=(--options runtime --entitlements packaging/macos/md2html.entitlements)
    fi
    signed=0
    # Mach-O executables only: skip .d files, build dirs, and the .bak
    # copies a deploy may have left beside them.
    for f in target/{{ profile }}/*; do
        [[ -f "$f" && -x "$f" ]] || continue
        case "$(file -b "$f")" in *Mach-O*) ;; *) continue;; esac
        if ! out="$(codesign -f -s "$identity" "${extra[@]}" "$f" 2>&1)"; then
            printf '%s\n' "$out" >&2
            printf "  \033[0;31m❌ codesign failed for %s\033[0m\n" "$f"
            exit 1
        fi
        signed=$((signed + 1))
    done
    printf "  \033[0;32m✅ signed %d binaries in target/{{ profile }} with %s\033[0m\n" "$signed" "$identity"

# Build the workspace and sign the result, in that order. Use instead
# of a bare `cargo build` while working on anything that asks macOS for
# a privacy-sensitive grant.

# Build the workspace and sign the binaries, in that order (macOS).
build-signed profile="debug":
    #!/usr/bin/env bash
    set -euo pipefail
    profile={{ quote(profile) }}
    if [[ "$profile" == "release" ]]; then cargo build --release --workspace; else cargo build --workspace; fi
    just sign-binaries "$profile"

# Create the local self-signed `md2html-dev` code-signing identity
# (macOS, idempotent, non-interactive). Only needed when no Developer
# ID is in the keychain.

# Create the local self-signed md2html-dev code-signing identity (macOS, idempotent).
setup-codesign:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[0;34m🔏 Binary code-signing setup (md2html-dev identity)\033[0m\n"
    if [[ "$(uname -s)" != "Darwin" ]]; then
        printf "  \033[0;36mℹ️  not macOS - nothing to do (binary signing is a macOS/TCC concern)\033[0m\n"
        exit 0
    fi
    existing="$(just _codesign-identity)"
    if [[ "$existing" == "Developer ID Application:"* ]]; then
        printf "  \033[0;32m✅ %s present - preferred; no local identity needed\033[0m\n" "$existing"
        exit 0
    fi
    all_ids="$(security find-identity -p codesigning)"
    if [[ "$all_ids" == *md2html-dev* ]]; then
        printf "  \033[0;32m✅ md2html-dev identity already present - nothing to change\033[0m\n"
        exit 0
    fi
    # A purely LOCAL self-signed identity: no secrets, no trust decision
    # outside this keychain, nothing to distribute. That is why it can be
    # a recipe at all - unlike commit signing, where key issuance is an
    # identity decision that needs a human.
    #
    # Three things here were established by testing, not by reading docs
    # (verified end to end in a scratch keychain):
    #
    #  1. The certificate needs the code-signing EKU set explicitly.
    #  2. The PKCS#12 needs a REAL password. Exporting with an empty one
    #     makes `security import` fail with "MAC verification failed
    #     during PKCS12 import (wrong password?)". The password is a
    #     transport detail for a file that lives for milliseconds.
    #  3. OpenSSL 3 writes PKCS#12 with algorithms macOS Security cannot
    #     read, so it needs `-legacy`. LibreSSL - which is what
    #     /usr/bin/openssl is on a stock Mac - REJECTS `-legacy` and is
    #     already compatible. Hence try-then-fall-back rather than
    #     assuming either.
    #
    # What is NOT needed: a trust setting. codesign signs happily with an
    # untrusted self-signed identity (verified), so this recipe never
    # calls add-trusted-cert and never raises an authorization dialog.
    workdir=$(mktemp -d)
    trap 'rm -rf "$workdir"' EXIT
    # Written with printf rather than a heredoc: a heredoc terminator
    # must sit at column 0, which would end the just recipe body.
    {
      printf '[ req ]\ndistinguished_name = dn\nprompt = no\nx509_extensions = codesign\n'
      printf '[ dn ]\nCN = md2html-dev\n'
      printf '[ codesign ]\nbasicConstraints = critical,CA:FALSE\n'
      printf 'keyUsage = critical,digitalSignature\n'
      printf 'extendedKeyUsage = critical,codeSigning\n'
    } > "$workdir/openssl.cnf"
    printf "  \033[0;36m→ generating a self-signed code-signing certificate\033[0m\n"
    if ! out="$(openssl req -x509 -newkey rsa:2048 -nodes \
        -keyout "$workdir/key.pem" -out "$workdir/cert.pem" \
        -days 3650 -config "$workdir/openssl.cnf" 2>&1)"; then
        printf '%s\n' "$out" >&2
        printf "  \033[0;31m❌ openssl could not create the certificate\033[0m\n"
        exit 1
    fi
    pass="md2html-setup-$$"
    # OpenSSL 3 needs -legacy; LibreSSL rejects it (see above): try one,
    # then the other, and show both errors when neither works.
    if ! legacy_err="$(openssl pkcs12 -export -legacy -inkey "$workdir/key.pem" -in "$workdir/cert.pem" \
        -name md2html-dev -out "$workdir/identity.p12" -passout "pass:$pass" 2>&1)"; then
        if ! plain_err="$(openssl pkcs12 -export -inkey "$workdir/key.pem" -in "$workdir/cert.pem" \
            -name md2html-dev -out "$workdir/identity.p12" -passout "pass:$pass" 2>&1)"; then
            printf '%s\n%s\n' "$legacy_err" "$plain_err" >&2
            printf "  \033[0;31m❌ could not export a PKCS#12 identity with %s\033[0m\n" "$(openssl version)"
            exit 1
        fi
    fi
    printf "  \033[0;36m→ importing into the login keychain\033[0m\n"
    keychain="$HOME/Library/Keychains/login.keychain-db"
    if ! security import "$workdir/identity.p12" -k "$keychain" -P "$pass" \
        -T /usr/bin/codesign -T /usr/bin/security; then
        printf "  \033[0;31m❌ keychain import failed\033[0m\n"
        exit 1
    fi
    # Without this, the FIRST codesign call raises a keychain-access
    # dialog per binary - the exact papercut this lane exists to remove.
    # `security` asks for the login keychain password here, once.
    if ! security set-key-partition-list -S apple-tool:,apple:,codesign: -s "$keychain" >/dev/null; then
        printf "  \033[0;31m❌ the key partition list was not set: codesign will ask for keychain access on each binary.\033[0m\n"
        printf "     Set it by hand: security set-key-partition-list -S apple-tool:,apple:,codesign: -s %s\n" "$keychain"
        exit 1
    fi
    printf "  \033[0;32m✅ md2html-dev identity created\033[0m\n"
    just doctor-codesign

# Binary signing is a macOS/TCC concern; there is nothing to verify
# elsewhere - and the bash variant below cannot start under the Windows
# just shell, so the OS split IS the guard.

# Is the code identity stable across rebuilds? (macOS only; no-op elsewhere)
[windows]
[linux]
doctor-codesign:
    @echo "doctor-codesign: not macOS - binary signing is a macOS concern, nothing to check"

# Is the code identity stable across rebuilds? The TCC question: a
# designated requirement containing `cdhash` means the grant dies on
# the next rebuild; one containing `certificate` survives it.

# Is the code identity stable across rebuilds? The TCC question.
[macos]
doctor-codesign:
    #!/usr/bin/env bash
    set -uo pipefail
    fail=0
    ok()   { printf "  \033[0;32m✅ %s\033[0m\n" "$1"; }
    bad()  { printf "  \033[0;31m❌ %s\033[0m\n" "$1"; fail=1; }
    info() { printf "  \033[0;36mℹ️  %s\033[0m\n" "$1"; }
    printf "\033[0;34m🩺 Binary code-signing doctor\033[0m\n"
    identity="$(just _codesign-identity)"
    if [[ "$identity" == "Developer ID Application:"* ]]; then
        ok "$identity (notarisable)"
    elif [[ "$identity" == "md2html-dev" ]]; then
        ok "md2html-dev (local; fine for development, not distributable)"
    elif [[ -n "$identity" ]]; then
        ok "$identity (from MD2HTML_CODESIGN_IDENTITY)"
    else
        bad "no signing identity (Developer ID or md2html-dev) - run: just setup-codesign"
    fi
    checked=0; hashed=()
    for dir in target/release target/debug; do
        [[ -d "$dir" ]] || continue
        for f in "$dir"/*; do
            [[ -f "$f" && -x "$f" ]] || continue
            case "$f" in *.bak|*.old|*.new|*.d) continue;; esac
            case "$(file -b "$f")" in *Mach-O*) ;; *) continue;; esac
            checked=$((checked + 1))
            # Capture, THEN match. `codesign ... | grep -q` is wrong
            # under pipefail: grep -q closes the pipe at the first match,
            # codesign dies on SIGPIPE, and a correctly-signed binary is
            # reported as ad-hoc.
            dr=$(codesign -d -r- "$f" 2>&1 | sed -n 's/^designated => //p')
            case "$dr" in
                *certificate*) ;;
                *) hashed+=("$f");;
            esac
        done
    done
    if (( checked == 0 )); then
        info "no built binaries under target/ - build first: just build-signed"
    elif (( ${#hashed[@]} == 0 )); then
        ok "all $checked built binaries are certificate-pinned (the grant survives rebuilds)"
    else
        bad "${#hashed[@]} of $checked built binaries are hash-pinned (ad-hoc): ${hashed[*]}"
        printf "     A cdhash designated requirement means the TCC grant dies on the next rebuild.\n"
        printf "     Fix:  just sign-binaries release   (or: just build-signed)\n"
    fi
    if (( fail )); then
        printf "\033[0;31m❌ binary signing incomplete - expect repeated macOS privacy prompts.\033[0m\n" >&2
        exit 1
    fi
    printf "\033[0;32m✅ binary code identity is stable.\033[0m\n"

# Re-sign already-installed binaries in a directory (safe while they
# are running: each is signed as a copy, then swapped in by rename).

# Re-sign already-installed binaries in a directory (safe while running).
resign-installed bindir="~/.cargo/bin":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "$(uname -s)" != "Darwin" ]]; then
        printf "  \033[0;36mℹ️  not macOS - binary signing is a macOS concern; nothing signed\033[0m\n"
        exit 0
    fi
    bindir="{{ bindir }}"; bindir="${bindir/#\~/$HOME}"
    printf "\033[0;34m🔏 Re-signing installed binaries in %s\033[0m\n" "$bindir"
    identity="$(just _codesign-identity)"
    if [[ -z "$identity" ]]; then
        printf "  \033[0;31m❌ no signing identity (Developer ID or md2html-dev) - run: just setup-codesign\033[0m\n"
        exit 1
    fi
    extra=(--identifier "{{ codesign_identifier }}")
    if [[ "$identity" == "Developer ID Application:"* ]]; then
        extra+=(--options runtime --entitlements packaging/macos/md2html.entitlements)
    fi
    # SIGN A COPY, THEN SWAP THE INODE. Signing in place rewrites a file
    # that running processes have mapped, and macOS then kills them as
    # signature-invalid. `mv` unlinks the old inode; anything still
    # running holds it open and finishes on it.
    signed=0; failed=0
    for f in "$bindir"/md2html*; do
        [[ -f "$f" && -x "$f" ]] || continue
        case "$f" in *.bak|*.old|*.new|*.sh) continue;; esac
        case "$(file -b "$f")" in *Mach-O*) ;; *) continue;; esac
        # Skip a binary already signed by this identity.  An unsigned one
        # makes `codesign -dvv` exit non-zero: it needs signing, and a real
        # problem with the file then fails the signing below, loudly.
        if signature=$(codesign -dvv "$f" 2>&1); then
            case "$signature" in *"Authority=$identity"*) continue;; esac
        fi
        staged="$f.new"
        if ! cp -p "$f" "$staged"; then
            printf "  \033[0;31m❌ could not stage a copy of %s\033[0m\n" "$(basename "$f")"
            failed=$((failed + 1))
            continue
        fi
        if out="$(codesign -f -s "$identity" "${extra[@]}" "$staged" 2>&1)" && mv "$staged" "$f"; then
            printf "  \033[0;32m✅ %s\033[0m\n" "$(basename "$f")"
            signed=$((signed + 1))
        else
            rm -f "$staged"
            printf '%s\n' "${out:-}" >&2
            printf "  \033[0;31m❌ %s\033[0m\n" "$(basename "$f")"
            failed=$((failed + 1))
        fi
    done
    printf "  re-signed %d, failed %d (already-signed binaries were skipped)\n" "$signed" "$failed"
    if (( failed )); then
        exit 1
    fi

# Push the Apple signing and notarization secrets from a gopass store to
# the repository's GitHub Actions secrets in one step, never by hand
# (docs/forge/CODE-SIGNING.md section 4 lists the paths). Files travel
# base64-encoded; release.yml decodes them. Re-run after a certificate
# or key rotates. Needs `gh` with admin and `gopass`.

# Push the Apple signing/notarization secrets from gopass to GitHub Actions secrets.
secrets-sync-apple prefix="keys/apple":
    #!/usr/bin/env bash
    set -euo pipefail
    command -v gopass >/dev/null || { echo "gopass is required (or set the secrets by hand: docs/forge/CODE-SIGNING.md section 4)"; exit 1; }
    repo="$(gh repo view --json nameWithOwner -q .nameWithOwner)"
    p="{{ prefix }}"
    echo "syncing Apple signing secrets from gopass $p/* to $repo"
    gopass show -o "$p/team-id"                    | gh secret set APPLE_TEAM_ID --repo "$repo"
    gopass show -o "$p/signing-identity"           | gh secret set APPLE_SIGNING_IDENTITY --repo "$repo"
    gopass show -o "$p/developer-id-p12-password"  | gh secret set APPLE_DEVELOPER_ID_P12_PASSWORD --repo "$repo"
    gopass cat "$p/developer-id-application.p12" | base64 | tr -d '\n' | gh secret set APPLE_DEVELOPER_ID_P12 --repo "$repo"
    gopass show -o "$p/asc-api-key-id"             | gh secret set APPLE_API_KEY_ID --repo "$repo"
    gopass show -o "$p/asc-api-issuer-id"          | gh secret set APPLE_API_ISSUER_ID --repo "$repo"
    gopass cat "$p/asc-api-key.p8" | base64 | tr -d '\n' | gh secret set APPLE_API_KEY_P8 --repo "$repo"
    # Seven secrets were just set: listing none of them is an error.
    listed="$(gh secret list --repo "$repo")"
    grep -E "^APPLE_" <<< "$listed"
