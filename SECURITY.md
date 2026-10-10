# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities **privately**, with *Report a
vulnerability* on the repository's
[Security tab](https://github.com/kuolas/raw-files/security).
Only you and the maintainer can see the report. Don't open a public issue,
pull request or comment for anything security-sensitive: the tracker is
public, and a report there discloses the problem before it can be fixed.

Include what you can: the affected component, steps to reproduce or a proof of
concept (a sample file that triggers the problem is ideal), and the impact as
you understand it.

You will get an acknowledgement within **7 days**. Please allow a reasonable
window for a fix before any public disclosure; you will be credited in the
release notes unless you prefer otherwise.

Bugs that are not security-sensitive go in the public
[issue tracker](https://github.com/kuolas/raw-files/issues).

## Supported versions

Nothing has been released yet. Once something is, only the latest release
will receive security fixes.

## Scope

Nothing in the crate reads untrusted input yet — it has no public API. A RAW
library exists to read files from cameras and from anyone who sends one, so
every byte it parses is untrusted input. Once it parses files, the most
valuable reports are malformed files that make it crash or panic, loop
without end, allocate far more memory than the file's size justifies, or read
outside the data it was given.
