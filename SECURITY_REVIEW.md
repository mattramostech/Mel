# Repository privacy review — 2026-09-26

## Outcome

No credential, private-key, signing-profile or concrete device-identifier
exposure was detected in the inspected repository content. This is **not** a
claim that nothing private has ever been published:

- **Published personal metadata:** a personal author email is present in all
  18 commits reachable from the fetched public GitHub refs. GitHub Actions
  run metadata also repeats that address. Its value is deliberately omitted.
- **Unpublished history requiring cleanup before publication:** local commit
  `ddd5b0582da0` records owner-specific physical-device/OS/Xcode/host details in
  `mel-skia/DECISIONS/009-real-device.md`, `DEBT.md` and `doc_context/mel_spec.md`.
  Its signing test fixture also reused the device's exact OS version. These
  details have been generalized in the working tree. The old commit still
  contains them and must not be pushed unchanged under the new privacy policy.
  This commit is absent from the remote refs retrieved during this audit;
  that does not prove it was never present in a deleted ref or external copy.
- **Local private artifacts:** the ignored development bundle contains a real
  provisioning profile and signing material/metadata. It is not a public
  distributable. No corresponding artifact path appears in inspected Git
  history. The root `.DS_Store` was untracked and is now ignored.
- **Unverified logs:** two historical CI executions have job output, but the
  log-download API returned HTTP 403 (administrator access required). Browser
  access was unavailable while the user was interacting with it. Their log
  contents have not been cleared by this audit.

## Scope and evidence

Reviewed the index (empty), tracked working files and nonignored untracked
files, preserving the in-progress 010 changes. Inspected filenames across all
local history: 51 distinct paths, with no tracked logs, signing assets,
development bundles, environment files, screenshots or other flagged artifacts.

Scanned all 20 local commits/refs/reflogs and all 179 blob/commit objects
present at the object-inventory snapshot (159 blobs, 20 commits), including
objects no longer reachable from branch tips. Tree entries and paths were
checked separately. No submodule or Git LFS content was present.

Cloned the public origin into a private temporary directory and explicitly
fetched PR heads. Retrieved four branches, zero tags, and three PR refs
(including an exposed merge ref): 18 commits and 89 blobs, with 39 historical
file paths. The public main tip was `bde7ab549720` at audit time. No Git refs
or history in the working repository were modified by this audit.

Used the official **Gitleaks 8.30.1** release, checking the downloaded archive
against its published SHA-256 checksum. Ran built-in credential rules over
local and remote histories with full redaction. Then scanned exported complete
objects and working-file snapshots using `.gitleaks.toml` (built-in rules plus
device, signing, home-directory and personal-email checks). The object scans
flagged the known personal email; no other rule found a secret. These scans
do not validate whether any credential would be active.

Compared eight known values from the existing local embedded provisioning
profile (device/profile/team/signing identifiers and certificate data) against
all exported local and remote objects without printing those values. No match.
No private keys were exported and no repository content was uploaded to a
third-party scanner; scanning ran locally.

GitHub public surfaces inspected:

| Surface | Result |
| --- | --- |
| Issues/PRs | Two PRs, both with empty bodies; no issue-only entries or attachments |
| Issue comments, PR review comments/reviews, commit comments | None returned |
| Releases/assets | No releases |
| Actions artifacts | None returned |
| Actions runs | Seven; five have no jobs, two completed a job with ten steps each |
| Public run metadata | Personal email repeated from commit metadata |
| Job logs | Not inspected: 403 for runs `31004312527` and `31004134884` |
| Deployments/Pages | No deployments; Pages disabled |
| Discussions | Disabled |
| Forks | None reported by GitHub |
| Wiki | No publicly cloneable wiki returned |

Public metadata responses were also scanned locally; only the already noted
personal email was detected. Empty inventories describe current API responses,
not proof that expired/deleted artifacts never existed. Third-party clones,
caches and private GitHub security/account settings are outside this evidence.

## Changes made

- Root `AGENTS.md`: mandatory privacy review before finishing each task and
  before any separately authorized publication; explicit coverage of docs,
  tests, commit metadata, logs, images, generated artifacts and remote history.
  Preserve the existing prohibition on agent commits/pushes/history rewrites.
- Root `.gitignore`: ignore local metadata, environment/signing files, build
  bundles, Xcode user state, raw diagnostics and crash artifacts repository-wide.
- `.gitleaks.toml`: retain default credential rules and add seven privacy rules.
  No finding was silenced with a broad allowlist.
- Generalized private environment observations in the three documents above;
  the signing fixture now uses a clearly generic supported OS version.

Validation: all seven added scanner rules detected synthetic fixtures kept
outside Git; 18 representative private artifact paths were confirmed ignored
while `.env.example` remains reviewable; all 12 signing tests passed. The final
working-file snapshot and empty staged diff passed the redacted scanner and
`git diff --check`. No executable/library behavior changed.

## Remaining recovery work

1. For future human commits, configure this repository to use the account's
   GitHub-provided noreply email. This audit did not change Git identity or
   account settings.
2. Before publishing the four local-only commits (006–009), prepare a sanitized
   replacement history that retains their code and the current 010 work, removes
   the private environment observations from **every** intermediate commit, and
   uses the chosen public author identity. A later cleanup commit alone is not
   enough. This audit did not rewrite those commits.
3. Removing the personal address already in public history requires an explicit
   history-rewrite plan covering all branches/tags/PR refs, coordination with
   GitHub for retained references/caches as needed, and separately authorized
   force-pushes. Existing external copies cannot be guaranteed erased.
4. With administrator access, download and inspect the two remaining CI logs;
   decide separately whether to delete retained logs/run metadata containing
   personal data. No CI logs, runs or other remote data were deleted here.

No credential rotation is indicated by the inspected evidence. If subsequent
log inspection finds an actual secret, revoke/rotate it before treating Git
cleanup as remediation. All findings here are reported without secret values.
