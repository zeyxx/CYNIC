"""Tests for NEAR smart contract"""

import os


class TestNearContract:
    """Test NEAR smart contract"""

    def test_contract_file_exists(self):
        """Test that Rust contract file exists"""
        contract_file = "contracts/governance.rs"
        assert os.path.exists(contract_file), f"Contract not found at {contract_file}"

    def test_contract_has_required_methods(self):
        """Test that contract has required methods"""
        contract_file = "contracts/governance.rs"
        assert os.path.exists(contract_file)

        with open(contract_file) as f:
            content = f.read()

        # Check for required methods
        required_methods = [
            "create_proposal",
            "vote",
            "get_proposal",
            "execute_proposal"
        ]

        for method in required_methods:
            assert f"pub fn {method}" in content, f"Missing method: {method}"

    def test_cargo_toml_exists(self):
        """Test that Cargo.toml exists"""
        cargo_file = "contracts/Cargo.toml"
        assert os.path.exists(cargo_file), f"Cargo.toml not found at {cargo_file}"

        with open(cargo_file) as f:
            content = f.read()

        assert "near-sdk" in content, "near-sdk dependency missing"
