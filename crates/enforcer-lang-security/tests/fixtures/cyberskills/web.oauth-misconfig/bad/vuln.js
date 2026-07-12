// Legacy "Sign in with Acme" OAuth client, pending security review.
//
// Two independent misconfigurations here (either one alone is a real
// finding; this fixture intentionally keeps both for coverage):
//   1. The implicit grant flow is still enabled (response_type=token),
//      so the access token comes back in the URL fragment instead of a
//      server-side code exchange.
//   2. The redirect_uri registered for the client is a wildcard, so any
//      subdomain (or path) under the scheme can receive the auth
//      response — SKILL.md's "Redirect URI Subdomain Bypass" scenario.

const oauthConfig = {
  client_id: "acme-web-client",
  response_type: "token",
  redirect_uri: "https://*.acme-app.example.com/callback",
  scope: "openid profile email",
};

function buildAuthorizeUrl(baseUrl) {
  const params = new URLSearchParams(oauthConfig);
  return `${baseUrl}/oauth/authorize?${params.toString()}`;
}

// A second, separate endpoint that builds its own redirect straight from
// the incoming request instead of the registered client's allowlisted
// value — an open redirect through the OAuth callback.
function handleLoginRedirect(req, res) {
  const redirect_uri = req.query.redirect;
  res.redirect(`https://auth.acme.example.com/oauth/authorize?response_type=code&client_id=acme-web-client&redirect_uri=${encodeURIComponent(redirect_uri)}`);
}

module.exports = { oauthConfig, buildAuthorizeUrl, handleLoginRedirect };
