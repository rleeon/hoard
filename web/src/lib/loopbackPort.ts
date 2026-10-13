/**
 * Sanitise the `?port=` the desktop app passes for the loopback login handoff.
 *
 * The value is spliced into `http://127.0.0.1:${port}/callback?access_token=…`,
 * so anything but a bare port number rewrites where the tokens go:
 * `?port=1@evil.com` makes `127.0.0.1:1` the userinfo and `evil.com` the host.
 * The app binds `127.0.0.1:0`, so the OS always hands it an unprivileged port;
 * accept only that shape and return `null` for everything else.
 */
export function loopbackPort(raw: string | null | undefined): string | null {
  if (!raw || !/^\d{1,5}$/.test(raw)) return null;
  const n = Number(raw);
  if (n < 1024 || n > 65535) return null;
  return String(n);
}
