'use strict';

const jwt = require('jsonwebtoken');

const SECRET = process.env.JWT_SIGNING_SECRET;

function verifyToken(token) {
  // Explicit algorithm allowlist (no "none"), and the secret is loaded
  // from the environment rather than hardcoded in source.
  return jwt.verify(token, SECRET, { algorithms: ['HS256'] });
}

function issueToken(payload) {
  return jwt.sign(payload, SECRET, { algorithm: 'HS256', expiresIn: '15m' });
}

module.exports = { verifyToken, issueToken };
