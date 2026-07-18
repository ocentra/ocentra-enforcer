# Terraform Rules

Static Terraform checks operate on `.tf` source without executing Terraform.

# Covered Rules

- `IAC-1.1`: S3 bucket resources must have a server-side encryption configuration.
- `IAC-1.2`: Security-group ingress must not allow `0.0.0.0/0`.
- `IAC-1.3`: Provider and resource bodies must not contain literal credentials or secrets.
- `IAC-1.6`: Required providers must pin an exact version.
- `IAC-1.7`: S3 remote-state backends must set `encrypt = true`.
