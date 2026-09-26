#[async_trait]
impl MeshStorage for InMemoryStorage {
    async fn publish_mesh_key(
        &self,
        mesh_ip: &str,
        public_key: &str,
    ) -> Result<MeshNode, StorageError> {
        let mut s = self.state.lock().await;
        // Upsert by mesh_ip
        let node = MeshNode {
            id: mesh_ip.to_string(),
            name: "".to_string(),
            public_key: public_key.to_string(),
            mesh_ip: mesh_ip.to_string(),
            external_endpoint: "".to_string(),
            last_seen: chrono::Utc::now().timestamp_millis().to_string(),
        };
        s.mesh_nodes.insert(mesh_ip.to_string(), node.clone());
        Ok(node)
    }

    async fn list_mesh_nodes(&self) -> Result<Vec<MeshNode>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.mesh_nodes.values().cloned().collect())
    }

    async fn list_mesh_routes(&self) -> Result<Vec<MeshRoute>, StorageError> {
        let s = self.state.lock().await;
        Ok(s.mesh_routes.clone())
    }
}
