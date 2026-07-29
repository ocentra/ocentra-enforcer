variable "region" {
  default = "us-east-1"
}

resource "aws_instance" "web" {
  ami           = "ami-123"
  instance_type = "t2.micro"
  count         = length(var.list)
}
