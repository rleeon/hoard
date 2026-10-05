#!/usr/bin/env python3
"""Servidor estático para Hoard web en local."""

import http.server
import os
import socket
import sys

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 3000
DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "build")

os.chdir(DIR)

handler = http.server.SimpleHTTPRequestHandler
handler.extensions_map.update({
    ".js": "application/javascript",
    ".css": "text/css",
    ".json": "application/json",
    ".svg": "image/svg+xml",
    ".woff2": "font/woff2",
    ".woff": "font/woff",
})

class SPAHandler(handler):
    def do_GET(self):
        path = self.translate_path(self.path)
        # GitHub Pages answers /download with download.html, the prerendered
        # page; only an unknown route gets the SPA shell.
        if not os.path.exists(path) and not os.path.splitext(self.path)[1]:
            route, _, query = self.path.partition("?")
            if os.path.isfile(self.translate_path(route) + ".html"):
                self.path = route + ".html" + ("?" + query if query else "")
            else:
                self.path = "/200.html"
        super().do_GET()

def get_ips():
    ips = []
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            ip = info[4][0]
            if not ip.startswith("127."):
                ips.append(ip)
    except:
        pass
    return sorted(set(ips))

ips = get_ips()
tailscale = [i for i in ips if i.startswith("100.")]
local = [i for i in ips if i.startswith("192.") or i.startswith("10.")]

print(f"Hoard web")
print(f"  Local:    http://localhost:{PORT}")
for ip in local:
    print(f"  Red:      http://{ip}:{PORT}")
for ip in tailscale:
    print(f"  Tailscale: http://{ip}:{PORT}")
print(f"Sirviendo: {DIR}")

httpd = http.server.HTTPServer(("0.0.0.0", PORT), SPAHandler)
try:
    httpd.serve_forever()
except KeyboardInterrupt:
    print("\nParado.")
