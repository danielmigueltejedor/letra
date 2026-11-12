# Security policy

## Reporting a vulnerability

Security reports for Letra belong to the **Nodalia maintainers**. If private vulnerability reporting is enabled, open [getnodalia/letra](https://github.com/getnodalia/letra) and select **Security → Advisories → Report a vulnerability**. Otherwise, ask the repository maintainers for a private transfer method using only a minimal, non-exploitable public summary. Do not disclose working exploits, sensitive logs, credentials, or malicious test files publicly.

Useful to include:
- the Letra version or commit, and the platform (macOS, Windows, Linux, BSD, web);
- what an attacker can do, and what they need (a crafted document, MCP or control-channel access, a local account…);
- steps or a sample file that reproduce it.

We'll acknowledge the report, keep you updated while we work on a fix, and credit you in the advisory unless you'd rather not be named.

## Supported versions

Fixes go into `main` and ship in the next release. Older releases don't get backported fixes.

## Scope

In scope: anything that lets a document, file, command, MCP or control-channel request, or network response read or write files it shouldn't, run code, leak data, or crash Letra and lose the user's work.

Crashes from hostile input count, because Letra treats every panic as a bug (see [Never crash](AGENTS.md)).
