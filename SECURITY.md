# Security

## Reporting a vulnerability

Please report security problems privately through GitHub:
**Security → Report a vulnerability** on
[farvisun/glaude](https://github.com/farvisun/glaude/security/advisories/new).
Do not open a public issue for them.

Include what you found, how to reproduce it, and which version and operating
system you used. You will get a reply within a week, and a fix or a plan
before the details are made public.

## Scope

Glaude runs the `claude` and `codex` CLIs on your computer and shows their
output. Of particular interest:

- Anything that lets content from a model reply, a file or a web page run
  code, open programs or reach the network outside what the user approved
  (the webview has a strict content security policy, and links never start
  programs).
- Leaks of credentials or proxy passwords. Glaude never reads the keychain or
  the CLIs' tokens, and proxy passwords are not logged.
- The background agent host (`glaude --host`) and its local socket.

## Supported versions

Fixes go into the latest release.
