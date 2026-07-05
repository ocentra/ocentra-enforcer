resource "aws_iam_policy" "reader" {
  name = "reader-policy"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect   = "Allow"
        Action   = ["s3:GetObject", "s3:ListBucket"]
        Resource = "arn:aws:s3:::my-company-logs/*"
      }
    ]
  })
}
