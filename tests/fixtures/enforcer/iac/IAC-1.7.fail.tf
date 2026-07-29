terraform {
  backend "s3" {
    bucket = "state-bucket"
    key    = "terraform.tfstate"
    region = "us-east-1"
  }
}
