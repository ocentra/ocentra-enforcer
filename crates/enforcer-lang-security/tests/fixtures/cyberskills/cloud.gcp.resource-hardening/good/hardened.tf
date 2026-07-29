resource "google_storage_bucket" "assets" {
  name     = "my-bucket"
  location = "US"
}

resource "google_storage_bucket_iam_member" "team_read" {
  bucket = google_storage_bucket.assets.name
  role   = "roles/storage.objectViewer"
  member = "group:data-team@example.com"
}

resource "google_project_iam_binding" "scoped_view" {
  project = "my-project"
  role    = "roles/viewer"
  members = [
    "user:alice@example.com",
    "serviceAccount:svc@my-project.iam.gserviceaccount.com",
  ]
}

resource "google_sql_database_instance" "primary" {
  name             = "primary-db"
  database_version = "POSTGRES_14"

  settings {
    tier = "db-custom-2-7680"

    ip_configuration {
      ipv4_enabled = false

      authorized_networks {
        name  = "office"
        value = "203.0.113.0/24"
      }
    }
  }
}

resource "google_compute_firewall" "allow_internal" {
  name    = "allow-internal-only"
  network = "default"

  allow {
    protocol = "tcp"
    ports    = ["443"]
  }

  source_ranges = ["10.0.0.0/8"]
}
