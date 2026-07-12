# python: requests call with certificate verification turned off
resp = requests.get("https://internal-api.example.com/status", verify=False)

# python: ssl module unverified context (equivalent to verify=False globally)
ctx = ssl._create_unverified_context()

# python: manual context hardening reversed
ctx2 = ssl.create_default_context()
ctx2.check_hostname = False
ctx2.verify_mode = ssl.CERT_NONE

# node.js: https client request options disabling cert verification
const options = { hostname: "internal-api.example.com", rejectUnauthorized: false };

# node.js: global env-var escape hatch disabling verification process-wide
process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";

# go: crypto/tls client config skipping verification
tlsConfig := &tls.Config{InsecureSkipVerify: true}

# php: cURL binding with peer verification disabled
curl_setopt($ch, CURLOPT_SSL_VERIFYPEER, 0);

# shell: curl invoked with the insecure flag against a health endpoint
curl -k https://internal-api.example.com/health

# shell: curl invoked with the long-form insecure flag
curl --insecure https://internal-api.example.com/health
