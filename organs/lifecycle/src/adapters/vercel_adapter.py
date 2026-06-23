from ..core.ports import ServerlessHostingPort
from ..core.entities import DeploymentTarget

class VercelAdapter(ServerlessHostingPort):
    async def deploy_project(self, target: DeploymentTarget, project_path: str) -> str:
        # TODO: Implement Vercel API
        return "vercel_deployment_url"
