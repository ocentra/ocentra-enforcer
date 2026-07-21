resource "aws_iam_user" "good_user" {
  name       = "good-user"
  password   = var.iam_user_password
  access_key = var.iam_access_key_id
}
