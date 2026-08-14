#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeshKeyValidationError {
    InvalidMeshIp,
    InvalidPublicKey,
}

impl MeshKeyValidationError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::InvalidMeshIp => "mesh_ip must be 1-20 characters",
            Self::InvalidPublicKey => "public_key must be 1-128 characters",
        }
    }
}

pub fn validate_mesh_key_publication(
    mesh_ip: &str,
    public_key: &str,
) -> Result<(), MeshKeyValidationError> {
    if mesh_ip.is_empty() || mesh_ip.len() > 20 {
        return Err(MeshKeyValidationError::InvalidMeshIp);
    }
    if public_key.is_empty() || public_key.len() > 128 {
        return Err(MeshKeyValidationError::InvalidPublicKey);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_mesh_key_publication_bounds() {
        assert!(validate_mesh_key_publication("10.20.0.2", "public-key").is_ok());
        assert_eq!(
            validate_mesh_key_publication("", "public-key"),
            Err(MeshKeyValidationError::InvalidMeshIp)
        );
        assert_eq!(
            validate_mesh_key_publication("10.20.0.2", ""),
            Err(MeshKeyValidationError::InvalidPublicKey)
        );
    }
}
