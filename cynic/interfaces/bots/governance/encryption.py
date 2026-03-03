"""Transparent encryption for governance models.

Provides encryption/decryption wrappers for sensitive governance data:
- community_token: API token for community authorization
- treasury_address: Blockchain address for treasury management

Architecture:
- Encryption happens transparently during model creation/update
- Decryption happens on-demand when accessing properties
- Encrypted values stored in database (_token_encrypted, _address_encrypted columns)
- Encryption keys fetched from Vault (no plaintext in code/env)
- Uses AES-256-GCM (authenticated encryption)
"""

from __future__ import annotations

import logging
from typing import Any, Optional

from cynic.kernel.security.encryption import EncryptionService

logger = logging.getLogger(__name__)


class EncryptedGovernanceData:
    """Mixin for transparent encryption/decryption in governance models."""

    _encryption_service: Optional[EncryptionService] = None
    _community_token_encrypted: Optional[str] = None
    _treasury_address_encrypted: Optional[str] = None

    @classmethod
    def set_encryption_service(cls, service: EncryptionService) -> None:
        """Set the global encryption service for all instances.

        Should be called during application startup.
        """
        cls._encryption_service = service
        logger.info("Encryption service set for governance models")

    async def encrypt_token(self, token: str) -> Optional[str]:
        """Encrypt community token for storage.

        Args:
            token: Plaintext token

        Returns:
            Encrypted token (hex-encoded) or None if encryption unavailable
        """
        if not token:
            return None

        if not self._encryption_service:
            logger.warning("Encryption service not configured, storing token plaintext")
            return token

        try:
            encrypted = await self._encryption_service.encrypt_string(
                token,
                key_id="governance-community-token",
            )
            logger.debug(f"Encrypted community token (length: {len(token)} -> {len(encrypted)})")
            return encrypted
        except Exception as e:
            logger.error(f"Failed to encrypt community token: {e}")
            raise

    async def decrypt_token(self, encrypted_token: Optional[str]) -> Optional[str]:
        """Decrypt community token for use.

        Args:
            encrypted_token: Encrypted token (hex-encoded)

        Returns:
            Plaintext token or None if not set
        """
        if not encrypted_token:
            return None

        if not self._encryption_service:
            logger.warning("Encryption service not configured, returning ciphertext as-is")
            return encrypted_token

        try:
            plaintext = await self._encryption_service.decrypt_string(
                encrypted_token,
                key_id="governance-community-token",
            )
            logger.debug("Decrypted community token successfully")
            return plaintext
        except ValueError as e:
            logger.error(f"Failed to decrypt community token: {e}")
            raise

    async def encrypt_address(self, address: str) -> Optional[str]:
        """Encrypt treasury address for storage.

        Args:
            address: Plaintext blockchain address

        Returns:
            Encrypted address (hex-encoded) or None if encryption unavailable
        """
        if not address:
            return None

        if not self._encryption_service:
            logger.warning("Encryption service not configured, storing address plaintext")
            return address

        try:
            encrypted = await self._encryption_service.encrypt_string(
                address,
                key_id="governance-treasury-address",
            )
            logger.debug(f"Encrypted treasury address (length: {len(address)} -> {len(encrypted)})")
            return encrypted
        except Exception as e:
            logger.error(f"Failed to encrypt treasury address: {e}")
            raise

    async def decrypt_address(self, encrypted_address: Optional[str]) -> Optional[str]:
        """Decrypt treasury address for use.

        Args:
            encrypted_address: Encrypted address (hex-encoded)

        Returns:
            Plaintext address or None if not set
        """
        if not encrypted_address:
            return None

        if not self._encryption_service:
            logger.warning("Encryption service not configured, returning ciphertext as-is")
            return encrypted_address

        try:
            plaintext = await self._encryption_service.decrypt_string(
                encrypted_address,
                key_id="governance-treasury-address",
            )
            logger.debug("Decrypted treasury address successfully")
            return plaintext
        except ValueError as e:
            logger.error(f"Failed to decrypt treasury address: {e}")
            raise


class EncryptedCommunityModel:
    """Helper for encrypted community model operations."""

    def __init__(self, encryption_service: Optional[EncryptionService] = None):
        self.encryption_service = encryption_service

    async def prepare_create(self, data: dict[str, Any]) -> dict[str, Any]:
        """Prepare data for community creation (encrypts sensitive fields).

        Args:
            data: Community data from request

        Returns:
            Data with encrypted token and address
        """
        prepared = data.copy()

        if self.encryption_service:
            # Encrypt community_token if present
            if "community_token" in prepared and prepared["community_token"]:
                try:
                    encrypted_token = await self.encryption_service.encrypt_string(
                        prepared["community_token"],
                        key_id="governance-community-token",
                    )
                    prepared["_community_token_encrypted"] = encrypted_token
                    # Remove plaintext from prepared data
                    prepared.pop("community_token", None)
                    logger.info("Community token encrypted for storage")
                except Exception as e:
                    logger.error(f"Failed to encrypt community token during create: {e}")
                    raise

            # Encrypt treasury_address if present
            if "treasury_address" in prepared and prepared["treasury_address"]:
                try:
                    encrypted_address = await self.encryption_service.encrypt_string(
                        prepared["treasury_address"],
                        key_id="governance-treasury-address",
                    )
                    prepared["_treasury_address_encrypted"] = encrypted_address
                    # Remove plaintext from prepared data
                    prepared.pop("treasury_address", None)
                    logger.info("Treasury address encrypted for storage")
                except Exception as e:
                    logger.error(f"Failed to encrypt treasury address during create: {e}")
                    raise

        return prepared

    async def prepare_response(self, community: Any) -> dict[str, Any]:
        """Prepare community data for API response (decrypts sensitive fields).

        Args:
            community: Community model instance

        Returns:
            Dictionary with decrypted sensitive fields
        """
        response = {
            "community_id": community.community_id,
            "community_name": community.community_name,
            "platform": community.platform,
            "token_symbol": community.token_symbol,
            "voting_period_hours": community.voting_period_hours,
            "execution_delay_hours": community.execution_delay_hours,
            "quorum_percentage": community.quorum_percentage,
            "approval_threshold_percentage": community.approval_threshold_percentage,
            "gasdf_enabled": community.gasdf_enabled,
            "cynic_enabled": community.cynic_enabled,
            "created_at": community.created_at.isoformat() if community.created_at else None,
        }

        # Decrypt community token if present
        if self.encryption_service and community._community_token_encrypted:
            try:
                decrypted_token = await self.encryption_service.decrypt_string(
                    community._community_token_encrypted,
                    key_id="governance-community-token",
                )
                response["community_token"] = decrypted_token
                logger.debug("Decrypted community token for response")
            except Exception as e:
                logger.warning(f"Failed to decrypt community token for response: {e}")
                response["community_token"] = None
        else:
            response["community_token"] = None

        # Decrypt treasury address if present
        if self.encryption_service and community._treasury_address_encrypted:
            try:
                decrypted_address = await self.encryption_service.decrypt_string(
                    community._treasury_address_encrypted,
                    key_id="governance-treasury-address",
                )
                response["treasury_address"] = decrypted_address
                logger.debug("Decrypted treasury address for response")
            except Exception as e:
                logger.warning(f"Failed to decrypt treasury address for response: {e}")
                response["treasury_address"] = None
        else:
            response["treasury_address"] = None

        return response
