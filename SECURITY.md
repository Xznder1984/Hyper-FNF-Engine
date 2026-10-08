# Security

## Reporting a vulnerability

Please **do not** open a public issue for security problems.

Contact the maintainers privately by opening a security advisory on GitHub:

<https://github.com/Xznder1984/Hyper-FNF-Engine/security/advisories/new>

or, if you cannot use GitHub, email the maintainers through the repository's
owner address listed in your commit history / GitHub profile.

You can expect:
- an acknowledgement within **48 hours**;
- a fix target communicated within **7 days** (or a reason it takes longer);
- a security advisory published once a fix is released.

## What we take seriously

- **Malicious engine/mod artifacts**: a compromised release, a tampered
  checksum, or a zip that escapes its install directory on extract.
- **Update channel attacks**: a forged update manifest that could push
  arbitrary code (mitigated by pinned SHA-256 digests and rollback).
- **Credential handling**: a leaked GitHub token from config files or logs.
- **Local privilege issues**: writing outside the user's data directory.

## Current mitigations

- HTTPS-only downloads; HTTP is refused at the policy layer (unit tested).
- SHA-256 verification of downloaded artifacts; verified-before-install.
- Zip extraction is zip-slip guarded (path-traversal unit tested).
- Engine artifacts are never executed or sourced outside their install root.
- Update manifests are pinned; failed updates roll back to the previous binary.
- GitHub tokens are stored via the OS keyring (keyring crate), never in files.

## Scope

These policies apply to the source of Hyper Engine in this repository and the
binaries it publishes. They do not govern third-party engines, mods, or the
SaraHUD project, which are subject to their own maintainers.