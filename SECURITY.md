# Security policy

Hoard stores other people's things. A bug here can lose data that took
someone hundreds of hours to make, or expose files they never meant to share.
Reports are welcome and taken seriously.

## Reporting a vulnerability

**Do not open a public issue for a security bug.** Use either:

- GitHub private advisory: <https://github.com/rleeon/hoard/security/advisories/new>
- Email: **support@hoard.services**, subject prefixed `SECURITY:`

Please include what you need to make the problem reproducible: affected
component and version, steps, and what an attacker gets out of it. A proof of
concept helps; a video is fine if that is faster for you.

Just be descriptive, if you use AI, say it.

If you want to encrypt the report, say so in a first plain message and we will
agree on a channel.

## What to expect

| Stage | Target |
|---|---|
| Acknowledgement of your report | 1-12 hours |
| First assessment (valid / not, severity) | 1 days |
| Fix released for a critical issue | 1-2 days |
| Fix released for other issues | 15 days |
| Public advisory | With the fix, or on agreement |

This is a small project run by one person, not a company with an on-call
rotation. If a deadline is going to slip we will tell you rather than go quiet.

## Coordinated disclosure

We ask you to give us the timeframes above before publishing. In exchange:

- We will not pursue legal action against anyone researching in good faith
  under this policy.
- You will be credited in the advisory and the changelog, unless you prefer
  not to be.
- We will tell you when the fix ships and confirm the issue is closed.

Good faith means: you did not access, modify or keep other people's data
beyond what was needed to demonstrate the flaw; you did not degrade the
service for others; and you did not use the finding for anything but the
report.

## Scope

In scope:

- `api.hoard.services` and `hoard.services`
- The desktop application, `hoardd`, the `hoard` CLI and `hoard-server`
- Anything in this repository

Out of scope:

- Denial of service through raw traffic volume
- Findings that only apply to an outdated release when a fixed one exists
- Reports produced by a scanner with no analysis of actual impact
- Social engineering of the maintainer or of users
- Missing hardening headers with no demonstrated exploit path

Self-hosted deployments are the operator's responsibility, but a flaw in the
software that affects them is in scope and we want to hear about it.

## Supported versions

Only the latest released version receives security fixes. Upgrading is the fix path, server or app.

## Handling of incidents affecting users

If an incident affects personal data of users of the managed service, we
notify the Spanish supervisory authority (AEPD) within 72 hours of becoming
aware, and the affected users without undue delay where the risk to them is
high. See the [Privacy Policy](https://hoard.services/legal/privacy).
