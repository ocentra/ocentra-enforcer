'use strict';

const jwt = require('jsonwebtoken');

function verifyToken(token) {
  // Accepts unsigned tokens: an attacker can forge a header with
  // {"alg":"none"} and strip the signature entirely.
  return jwt.verify(token, '', { algorithms: ['none'] });
}

function issueToken(payload) {
  // Hardcoded, short signing secret committed straight to source.
  return jwt.sign(payload, 'secret123');
}

module.exports = { verifyToken, issueToken };
