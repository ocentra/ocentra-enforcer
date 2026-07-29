resource "google_storage_bucket" "assets" {
  name     = "my-bucket"
  location = "US"
}

resource "google_storage_bucket_iam_member" "public_read" {
  bucket = google_storage_bucket.assets.name
  role   = "roles/storage.objectViewer"
  member = "allUsers"
}

resource "google_project_iam_binding" "public_view" {
  project = "my-project"
  role    = "roles/viewer"
  members = [
    "allAuthenticatedUsers",
  ]
}

resource "google_sql_database_instance" "primary" {
  name             = "primary-db"
  database_version = "POSTGRES_14"

  settings {
    tier = "db-custom-2-7680"

    ip_configuration {
      ipv4_enabled = true

      authorized_networks {
        name  = "any"
        value = "0.0.0.0/0"
      }
    }
  }
}

resource "google_compute_firewall" "allow_all" {
  name    = "allow-all-ingress"
  network = "default"

  allow {
    protocol = "tcp"
    ports    = ["0-65535"]
  }

  source_ranges = ["0.0.0.0/0"]
}
