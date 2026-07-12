"""Legacy authentication and encryption helpers pending migration.

This module also documents the equivalent call sites in other languages
(kept as string constants here so the cross-language migration checklist
and its regression fixtures live next to the code they describe).
"""
import hashlib

from Crypto.Cipher import ARC4, DES3


def hash_password(password: str) -> str:
    """Weak: MD5 is not suitable for password hashing."""
    return hashlib.md5(password.encode()).hexdigest()


def legacy_checksum(data: bytes) -> str:
    """Weak: SHA1 is broken for collision resistance."""
    return hashlib.sha1(data).hexdigest()


def encrypt_legacy_record(key: bytes, data: bytes) -> bytes:
    """Weak: 3DES in ECB mode leaks structural plaintext patterns."""
    cipher = DES3.new(key, DES3.MODE_ECB)
    return cipher.encrypt(data)


def encrypt_stream_legacy(key: bytes, data: bytes) -> bytes:
    """Weak: RC4 is a broken stream cipher."""
    cipher = ARC4.new(key)
    return cipher.encrypt(data)


# Equivalent Java call sites captured for the migration checklist:
JAVA_MD5_SNIPPET = 'MessageDigest.getInstance("MD5")'
JAVA_SHA1_SNIPPET = 'MessageDigest.getInstance("SHA1")'
JAVA_DES_CIPHER_SNIPPET = 'Cipher.getInstance("DES/ECB/PKCS5Padding")'
JAVA_AES_ECB_SNIPPET = 'Cipher.getInstance("AES/ECB/PKCS5Padding")'

# Equivalent Node.js call sites captured for the migration checklist:
NODE_MD5_SNIPPET = 'crypto.createHash("md5")'
NODE_SHA1_SNIPPET = "crypto.createHash('sha1')"
NODE_RC4_CIPHER_SNIPPET = "crypto.createCipheriv('rc4', key, null)"

# Insecure randomness: Math.random() used to mint a password-reset token.
RESET_TOKEN_JS_SNIPPET = "const resetToken = Math.random().toString(36).slice(2);"
