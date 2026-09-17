#!/usr/bin/env python3
"""Sign access tokens the cloud server accepts, against a JWKS of our own.

The local rehearsal runs real clients and the real server with no Supabase in
the loop: the server's JWKS URL points at the jwks.json this writes, and the
clients get a token signed with the matching key. Standard library plus the
openssl binary, nothing to install.

    mint-jwt.py DIR init                 key.pem + jwks.json in DIR
    mint-jwt.py DIR token SUB EMAIL      prints a token valid for 30 days
"""
import base64
import json
import subprocess
import sys
import time
from pathlib import Path


def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def init(d: Path) -> None:
    d.mkdir(parents=True, exist_ok=True)
    key = d / "key.pem"
    if not key.exists():
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out", str(key)],
            check=True, capture_output=True,
        )
    modulus = subprocess.run(
        ["openssl", "rsa", "-in", str(key), "-noout", "-modulus"],
        check=True, capture_output=True, text=True,
    ).stdout.strip().split("=", 1)[1]
    n = bytes.fromhex(modulus)
    jwks = {"keys": [{"kty": "RSA", "kid": "local", "use": "sig", "alg": "RS256",
                      "n": b64url(n), "e": "AQAB"}]}
    (d / "jwks.json").write_text(json.dumps(jwks))


def token(d: Path, sub: str, email: str) -> str:
    header = {"alg": "RS256", "typ": "JWT", "kid": "local"}
    now = int(time.time())
    claims = {"sub": sub, "email": email, "role": "authenticated", "aud": "authenticated",
              "iat": now, "exp": now + 30 * 86400, "user_metadata": {"full_name": email}}
    signing_input = f"{b64url(json.dumps(header).encode())}.{b64url(json.dumps(claims).encode())}"
    sig = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", str(d / "key.pem")],
        input=signing_input.encode(), check=True, capture_output=True,
    ).stdout
    return f"{signing_input}.{b64url(sig)}"


if __name__ == "__main__":
    directory = Path(sys.argv[1])
    if sys.argv[2] == "init":
        init(directory)
    else:
        print(token(directory, sys.argv[3], sys.argv[4]))
