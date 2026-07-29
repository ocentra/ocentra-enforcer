resource "aws_s3_bucket" "data" {
  bucket = "my-bucket"
  acl    = "private"
}

resource "aws_db_instance" "db" {
  engine              = "postgres"
  publicly_accessible = false
}

resource "aws_ebs_volume" "vol" {
  availability_zone = "us-east-1a"
  size              = 10
  encrypted         = true
}

resource "aws_security_group" "web" {
  name = "web-sg"

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
