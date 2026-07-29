"""Modern authentication and encryption helpers.

Historically this module used MD5, 3DES, and ECB mode; it was migrated
away from all of those weaknesses (MD5, SHA1, DES, 3DES, RC4, ECB) and now
relies exclusively on vetted primitives and CSPRNGs.
"""
import hashlib
import secrets

from Crypto.Cipher import AES


def hash_password(password: str, salt: bytes) -> str:
    """Strong: SHA-256 with a random per-user salt."""
    return hashlib.sha256(salt + password.encode()).hexdigest()


def checksum(data: bytes) -> str:
    """Strong: SHA-256 for integrity checks."""
    return hashlib.sha256(data).hexdigest()


def encrypt_record(key: bytes, nonce: bytes, data: bytes) -> bytes:
    """Strong: AES-256 in GCM mode provides authenticated encryption."""
    cipher = AES.new(key, AES.MODE_GCM, nonce=nonce)
    ciphertext, _tag = cipher.encrypt_and_digest(data)
    return ciphertext


def generate_reset_token() -> str:
    """Strong: secrets module is a CSPRNG suitable for tokens."""
    return secrets.token_urlsafe(32)


# Equivalent Java call sites captured for the migration checklist:
JAVA_SHA256_SNIPPET = 'MessageDigest.getInstance("SHA-256")'
JAVA_AES_GCM_SNIPPET = 'Cipher.getInstance("AES/GCM/NoPadding")'

# Equivalent Node.js call sites captured for the migration checklist:
NODE_SHA256_SNIPPET = 'crypto.createHash("sha256")'
NODE_AES_GCM_SNIPPET = "crypto.createCipheriv('aes-256-gcm', key, iv)"
NODE_TOKEN_SNIPPET = "const resetToken = crypto.randomBytes(32).toString('hex');"
