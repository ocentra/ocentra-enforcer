resource "aws_iam_user" "bad_user" {
  name          = "bad-user"
  password      = "SuperSecretPassword123!"
  access_key_id = "AKIAXXXXXXXXXXXXXXXX"
}
