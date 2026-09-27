# Repository instructions

This is a public, open-source repository. Treat everything in Git, pull
requests, issues, CI output, releases and shared artifacts as public.

## Working agreements

- For Mel implementation work, follow `mel-skia/doc_context/CONTEXT.md`,
  `ROADMAP.md` and `mel_spec.md`. Keep repository documentation in English.
- Never create commits, merge, push, publish artifacts, or rewrite Git history.
  Leave changes for human review. A clean scan is not permission to publish.

## Mandatory privacy review

Before finishing **every task**, review tracked changes, staged content and
new/untracked files for secrets and private metadata. Repeat the review on the
exact content proposed for any explicitly authorized external publication.
Do not limit the check to the files you intended to edit.

- Never include real tokens, passwords, authorization headers, cookies, private
  keys, signing certificates, provisioning profiles or local environment files.
- Never include real iPhone/device identifiers (UDID, serial, IMEI, ECID,
  CoreDevice or Simulator UUID), device names, Apple account/team identifiers,
  signing identities/fingerprints, profile identifiers, personal email addresses,
  local usernames/home paths, IP addresses or machine names.
- Keep the owner's hardware and Xcode/account environment private, including
  exact physical-device model/OS/host details from local tests. Use generic
  descriptions and clearly synthetic fixtures. Public SDK requirements and
  generic Simulator examples are fine when they do not identify the owner.
- Treat raw tool output, screenshots, crash reports, device listings,
  `devicectl`/`security`/`codesign`/Xcode logs, `.app`/`.ipa` bundles, debug
  symbols and scanner reports as private until individually inspected and
  sanitized. Do not attach or commit them just because a command succeeded.
- Keep private audit data outside the working tree. Do not print matched
  secrets while investigating: report only category, relative path, line and
  commit ID. Use full redaction in scanners, including their saved reports.

At minimum:

1. Inspect `git status --short`, `git diff`, `git diff --cached`, and the files
   from `git ls-files --others --exclude-standard`. Check filenames, binary
   artifacts and `.gitignore` coverage as well as text. Avoid emitting raw
   suspect content into the conversation or logs.
2. Run Gitleaks with this repository's `.gitleaks.toml`, `--redact=100` and
   `--ignore-gitleaks-allow` against staged changes and a temporary snapshot
   containing **all tracked working files plus nonignored untracked files**.
   Include tracked files even if a new ignore rule matches them. Do not follow
   symlinks into private directories. Review sensitive metadata separately:
   a secret scanner does not establish that logs or personal data are safe.
3. For a publication/history audit, also scan all local refs/reflogs and freshly
   retrieved remote branches, tags and PR refs. Inspect commit authors and
   messages, CI logs/artifacts and releases within the requested scope. Local
   remote-tracking refs alone do not prove what is published. Example:
   `gitleaks git --log-opts="--all --reflog" --redact=100 --ignore-gitleaks-allow .`
4. Fix working-tree leaks and use obvious placeholders/synthetic test data.
   `.gitignore` does not untrack a file or erase prior commits. Do not suppress
   findings with broad allowlists or silently omit files that could be published.
5. Report the scope checked, findings (without values), fixes and any unverified
   surfaces. If a scanner or remote access is unavailable, say so; never claim
   a complete audit or guarantee that nothing has ever leaked.

If exposure is found in existing commits or remote artifacts, explain which
surface contains it. Credential revocation, history rewriting/force-pushing,
remote deletion and account changes require a separate, concrete recovery plan
and explicit user authorization. Continue safe local cleanup meanwhile.

## Code Review Rules

Flag secrets, owner/device/account metadata and unreviewed logs or generated
artifacts as blocking publication issues, including those in docs, tests,
comments, commit messages and attachments. Verify that privacy review evidence
describes both scanned content and limitations; do not accept "ignored now"
as proof of removal from history.
