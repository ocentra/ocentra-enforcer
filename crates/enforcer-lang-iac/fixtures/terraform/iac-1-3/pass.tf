provider "aws" {
  # Credentials are sourced from the environment / instance role, never
  # hardcoded in source.
  region = "us-east-1"
}
