#!/usr/bin/env python3
"""The one Supabase Auth call a client makes on its own: refreshing a session.

1.0.4 renews its token at start-up whatever the expiry says, so a rehearsal
without Supabase needs something to answer. Each refresh token names a file in
/tokens holding the access token minted for it (mint-jwt.py); the answer hands
it back with the same refresh token, as GoTrue's shape.
"""
import json
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

TOKENS = Path("/tokens")


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        body = json.loads(self.rfile.read(length) or b"{}")
        refresh = str(body.get("refresh_token", ""))
        path = TOKENS / f"{refresh}.json"
        if not self.path.startswith("/auth/v1/token") or "/" in refresh or not path.exists():
            self.send_response(400)
            self.end_headers()
            self.wfile.write(b'{"error":"invalid_grant"}')
            return
        stored = json.loads(path.read_text())
        answer = {
            "access_token": stored["access_token"],
            "refresh_token": refresh,
            "token_type": "bearer",
            "expires_in": 3600,
            "expires_at": int(time.time()) + 3600,
            "user": {"id": stored["sub"], "email": stored["email"], "aud": "authenticated"},
        }
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(answer).encode())


HTTPServer(("0.0.0.0", 80), Handler).serve_forever()
