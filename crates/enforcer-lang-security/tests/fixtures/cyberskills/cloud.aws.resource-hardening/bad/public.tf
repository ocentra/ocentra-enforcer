resource "aws_s3_bucket" "data" {
  bucket = "my-bucket"
  acl    = "public-read"
}

resource "aws_db_instance" "db" {
  engine              = "postgres"
  publicly_accessible = true
}

resource "aws_ebs_volume" "vol" {
  availability_zone = "us-east-1a"
  size              = 10
}

resource "aws_security_group" "web" {
  name = "web-sg"

  ingress {
    from_port   = 3389
    to_port     = 3389
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
