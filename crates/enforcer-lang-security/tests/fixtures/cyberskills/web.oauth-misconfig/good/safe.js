// "Sign in with Acme" OAuth client, hardened per the current guidance:
// authorization-code flow with PKCE, an exact-match allowlisted
// redirect_uri (registered out-of-band, never derived from the request),
// and a random per-request state value bound to the user's session.

const crypto = require("crypto");

// Exact-match literal, registered with the provider ahead of time. Never
// built from a query/body/params value.
const REDIRECT_URI = "https://app.acme-app.example.com/callback";

function buildAuthorizeUrl(baseUrl, sessionState) {
  const verifier = crypto.randomBytes(32).toString("base64url");
  const challenge = crypto
    .createHash("sha256")
    .update(verifier)
    .digest("base64url");

  const params = new URLSearchParams({
    client_id: "acme-web-client",
    response_type: "code",
    redirect_uri: REDIRECT_URI,
    scope: "openid profile email",
    state: sessionState,
    code_challenge: challenge,
    code_challenge_method: "S256",
  });

  return { url: `${baseUrl}/oauth/authorize?${params.toString()}`, verifier };
}

function handleLoginRedirect(req, res) {
  // The callback always resolves to the constant above; nothing here is
  // read from the request.
  res.redirect(REDIRECT_URI);
}

module.exports = { REDIRECT_URI, buildAuthorizeUrl, handleLoginRedirect };
