# python: requests call with certificate verification left on (default)
resp = requests.get("https://internal-api.example.com/status", verify=True)

# python: requests call pinned to a custom CA bundle, verification still on
resp2 = requests.get("https://internal-api.example.com/status", verify="/etc/ssl/certs/ca-bundle.pem")

# python: standard hardened context, hostname checking left on
ctx = ssl.create_default_context()
ctx.check_hostname = True
ctx.verify_mode = ssl.CERT_REQUIRED

# node.js: https client request options with verification left on
const options = { hostname: "internal-api.example.com", rejectUnauthorized: true };

# node.js: env-var escape hatch left at its secure default
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "1";

# go: crypto/tls client config with verification left on
tlsConfig := &tls.Config{InsecureSkipVerify: false}

# php: cURL binding with peer verification left on
curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, true);

# shell: curl invoked without any insecure flag
curl https://internal-api.example.com/health

# shell: curl with unrelated short flags, none of which disable verification
curl -sSL https://internal-api.example.com/health

# prose mentioning the word insecure with no curl invocation on the line
# this configuration used to be insecure but has since been hardened
