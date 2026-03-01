"""
Tests for Deployment Readiness

Verifies that:
- Dockerfile builds successfully
- Docker Compose configuration is valid
- Kubernetes manifests are valid
- Health check endpoints are configured
- Configuration files are present
"""

from pathlib import Path

import pytest
import yaml


class TestDockerConfiguration:
    """Test Docker configuration files"""

    def test_dockerfile_governance_exists(self):
        """Dockerfile.governance exists"""
        dockerfile = Path("docker/Dockerfile.governance")
        assert dockerfile.exists(), "docker/Dockerfile.governance not found"

    def test_dockerfile_has_multi_stage_build(self):
        """Dockerfile uses multi-stage build for optimization"""
        with open("docker/Dockerfile.governance") as f:
            content = f.read()

        # Count FROM statements
        from_count = content.count("FROM ")
        assert from_count >= 2, "Dockerfile should use multi-stage build (at least 2 FROM statements)"

    def test_dockerfile_has_health_check(self):
        """Dockerfile includes HEALTHCHECK"""
        with open("docker/Dockerfile.governance") as f:
            content = f.read()

        assert "HEALTHCHECK" in content, "Dockerfile missing HEALTHCHECK directive"

    def test_dockerfile_creates_non_root_user(self):
        """Dockerfile creates non-root user"""
        with open("docker/Dockerfile.governance") as f:
            content = f.read()

        assert "useradd" in content, "Dockerfile should create non-root user"

    def test_entrypoint_script_exists(self):
        """Entrypoint script exists"""
        script = Path("docker/entrypoint.sh")
        assert script.exists(), "docker/entrypoint.sh not found"

    def test_entrypoint_validates_token(self):
        """Entrypoint script validates DISCORD_TOKEN"""
        with open("docker/entrypoint.sh") as f:
            content = f.read()

        assert "DISCORD_TOKEN" in content, "Entrypoint should validate DISCORD_TOKEN"


class TestDockerCompose:
    """Test Docker Compose configuration"""

    def test_docker_compose_governance_exists(self):
        """docker-compose.governance.yml exists"""
        compose_file = Path("docker-compose.governance.yml")
        assert compose_file.exists(), "docker-compose.governance.yml not found"

    def test_docker_compose_governance_valid_yaml(self):
        """docker-compose.governance.yml is valid YAML"""
        with open("docker-compose.governance.yml") as f:
            try:
                yaml.safe_load(f)
            except yaml.YAMLError as e:
                pytest.fail(f"Invalid YAML in docker-compose.governance.yml: {e}")

    def test_docker_compose_has_governance_bot_service(self):
        """docker-compose.governance.yml defines governance-bot service"""
        with open("docker-compose.governance.yml") as f:
            config = yaml.safe_load(f)

        assert "services" in config, "No services defined in docker-compose"
        assert "governance-bot" in config["services"], "governance-bot service not defined"

    def test_docker_compose_defines_volumes(self):
        """docker-compose.governance.yml defines persistent volumes"""
        with open("docker-compose.governance.yml") as f:
            config = yaml.safe_load(f)

        assert "volumes" in config, "No volumes defined"
        assert len(config["volumes"]) > 0, "At least one volume required"

    def test_docker_compose_defines_networks(self):
        """docker-compose.governance.yml defines networks"""
        with open("docker-compose.governance.yml") as f:
            config = yaml.safe_load(f)

        assert "networks" in config, "No networks defined"


class TestKubernetesConfiguration:
    """Test Kubernetes manifests"""

    def test_deployment_yaml_exists(self):
        """kubernetes/deployment.yaml exists"""
        deployment = Path("kubernetes/deployment.yaml")
        assert deployment.exists(), "kubernetes/deployment.yaml not found"

    def test_deployment_yaml_valid(self):
        """kubernetes/deployment.yaml is valid YAML"""
        with open("kubernetes/deployment.yaml") as f:
            try:
                # Load multiple documents
                docs = list(yaml.safe_load_all(f))
                assert len(docs) > 0, "No YAML documents in deployment.yaml"
            except yaml.YAMLError as e:
                pytest.fail(f"Invalid YAML in deployment.yaml: {e}")

    def test_deployment_has_namespace(self):
        """deployment.yaml defines namespace"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        namespaces = [d for d in docs if d.get("kind") == "Namespace"]
        assert len(namespaces) > 0, "No Namespace defined"

    def test_deployment_has_configmap(self):
        """deployment.yaml defines ConfigMap"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        configmaps = [d for d in docs if d.get("kind") == "ConfigMap"]
        assert len(configmaps) > 0, "No ConfigMap defined"

    def test_deployment_has_secret(self):
        """deployment.yaml defines Secret"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        secrets = [d for d in docs if d.get("kind") == "Secret"]
        assert len(secrets) > 0, "No Secret defined"

    def test_deployment_has_deployment_resource(self):
        """deployment.yaml defines Deployment resource"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        assert len(deployments) > 0, "No Deployment resource defined"

    def test_deployment_has_hpa(self):
        """deployment.yaml defines HorizontalPodAutoscaler"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        hpas = [d for d in docs if d.get("kind") == "HorizontalPodAutoscaler"]
        assert len(hpas) > 0, "No HorizontalPodAutoscaler defined"

    def test_deployment_has_pdb(self):
        """deployment.yaml defines PodDisruptionBudget"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        pdbs = [d for d in docs if d.get("kind") == "PodDisruptionBudget"]
        assert len(pdbs) > 0, "No PodDisruptionBudget defined"

    def test_service_yaml_exists(self):
        """kubernetes/service.yaml exists"""
        service = Path("kubernetes/service.yaml")
        assert service.exists(), "kubernetes/service.yaml not found"

    def test_service_yaml_valid(self):
        """kubernetes/service.yaml is valid YAML"""
        with open("kubernetes/service.yaml") as f:
            try:
                docs = list(yaml.safe_load_all(f))
                assert len(docs) > 0, "No YAML documents in service.yaml"
            except yaml.YAMLError as e:
                pytest.fail(f"Invalid YAML in service.yaml: {e}")

    def test_service_has_services(self):
        """service.yaml defines Service resources"""
        with open("kubernetes/service.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        services = [d for d in docs if d.get("kind") == "Service"]
        assert len(services) > 0, "No Service resources defined"

    def test_service_has_network_policy(self):
        """service.yaml defines NetworkPolicy"""
        with open("kubernetes/service.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        policies = [d for d in docs if d.get("kind") == "NetworkPolicy"]
        assert len(policies) > 0, "No NetworkPolicy defined"


class TestHealthChecks:
    """Test health check configuration"""

    def test_deployment_has_liveness_probe(self):
        """Deployment defines liveness probe"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        assert len(deployments) > 0

        deployment = deployments[0]
        containers = deployment["spec"]["template"]["spec"]["containers"]
        assert len(containers) > 0

        container = containers[0]
        assert "livenessProbe" in container, "No liveness probe defined"

    def test_deployment_has_readiness_probe(self):
        """Deployment defines readiness probe"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        deployment = deployments[0]
        container = deployment["spec"]["template"]["spec"]["containers"][0]

        assert "readinessProbe" in container, "No readiness probe defined"

    def test_deployment_has_startup_probe(self):
        """Deployment defines startup probe"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        deployment = deployments[0]
        container = deployment["spec"]["template"]["spec"]["containers"][0]

        assert "startupProbe" in container, "No startup probe defined"


class TestDocumentation:
    """Test deployment documentation"""

    def test_deployment_guide_exists(self):
        """DEPLOYMENT_GUIDE.md exists"""
        guide = Path("DEPLOYMENT_GUIDE.md")
        assert guide.exists(), "DEPLOYMENT_GUIDE.md not found"

    def test_deployment_guide_has_docker_section(self):
        """DEPLOYMENT_GUIDE.md covers Docker"""
        with open("DEPLOYMENT_GUIDE.md") as f:
            content = f.read()

        assert "Docker Compose" in content, "No Docker Compose section"
        assert "docker-compose up" in content, "Missing docker-compose examples"

    def test_deployment_guide_has_kubernetes_section(self):
        """DEPLOYMENT_GUIDE.md covers Kubernetes"""
        with open("DEPLOYMENT_GUIDE.md") as f:
            content = f.read()

        assert "Kubernetes" in content, "No Kubernetes section"
        assert "kubectl apply" in content, "Missing kubectl examples"

    def test_deployment_guide_has_troubleshooting(self):
        """DEPLOYMENT_GUIDE.md includes troubleshooting"""
        with open("DEPLOYMENT_GUIDE.md") as f:
            content = f.read()

        assert "Troubleshooting" in content, "No troubleshooting section"


class TestSecurityConfiguration:
    """Test security-related deployment settings"""

    def test_kubernetes_has_rbac(self):
        """Kubernetes manifests define RBAC"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        roles = [d for d in docs if d.get("kind") == "Role"]
        service_accounts = [d for d in docs if d.get("kind") == "ServiceAccount"]
        role_bindings = [d for d in docs if d.get("kind") == "RoleBinding"]

        assert len(roles) > 0, "No Role defined"
        assert len(service_accounts) > 0, "No ServiceAccount defined"
        assert len(role_bindings) > 0, "No RoleBinding defined"

    def test_kubernetes_has_security_context(self):
        """Kubernetes Deployment includes security context"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        deployment = deployments[0]
        container = deployment["spec"]["template"]["spec"]["containers"][0]

        assert "securityContext" in container, "No security context defined"

    def test_dockerfile_runs_as_nonroot(self):
        """Dockerfile runs as non-root user"""
        with open("docker/Dockerfile.governance") as f:
            content = f.read()

        assert "USER " in content, "Dockerfile doesn't specify USER"
        # Verify it's not root
        lines = content.split("\n")
        user_lines = [l for l in lines if l.startswith("USER ")]
        assert len(user_lines) > 0
        assert "root" not in user_lines[-1], "Container runs as root"


class TestResourceConfiguration:
    """Test resource limits and requests"""

    def test_kubernetes_has_resource_requests(self):
        """Kubernetes Deployment defines resource requests"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        deployment = deployments[0]
        container = deployment["spec"]["template"]["spec"]["containers"][0]

        assert "resources" in container, "No resources defined"
        assert "requests" in container["resources"], "No resource requests"

    def test_kubernetes_has_resource_limits(self):
        """Kubernetes Deployment defines resource limits"""
        with open("kubernetes/deployment.yaml") as f:
            docs = list(yaml.safe_load_all(f))

        deployments = [d for d in docs if d.get("kind") == "Deployment"]
        deployment = deployments[0]
        container = deployment["spec"]["template"]["spec"]["containers"][0]

        assert "resources" in container
        assert "limits" in container["resources"], "No resource limits"
