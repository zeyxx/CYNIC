# Terraform — CYNIC Cloud Foundation (GCP)

resource "google_cloud_run_service" "cynic_kernel" {
  name     = "cynic-kernel"
  location = var.region

  template {
    spec {
      containers {
        image = "gcr.io/${var.project_id}/cynic-organism:latest"
        env {
          name  = "CONSCIOUSNESS_LEVEL"
          value = "MICRO"
        }
        resources {
          limits = {
            cpu    = "1000m"
            memory = "2Gi"
          }
        }
      }
    }
  }

  traffic {
    percent         = 100
    latest_revision = true
  }
}
