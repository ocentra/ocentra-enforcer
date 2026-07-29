resource "aws_s3_bucket_server_side_encryption_configuration" "good_bucket" {
  bucket = "good-bucket"

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}
