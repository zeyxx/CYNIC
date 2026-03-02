"""Formal specification tests for P10 probes service."""

import pytest


class TestFormalSpecListFiltering:
    """Formal Spec: List filtering by status."""

    @pytest.mark.integration
    def test_list_filters_by_status_pending(self, sample_proposals, self_prober):
        """
        Spec: list_probes("PENDING") returns ONLY proposals with status="PENDING"
        """
        # This test will fail until ProbesService is implemented
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        pending = service.list_probes("PENDING")

        assert len(pending) == 5
        assert all(p["status"] == "PENDING" for p in pending)

    @pytest.mark.integration
    def test_list_filters_by_status_applied(self, sample_proposals, self_prober):
        """
        Spec: list_probes("APPLIED") returns ONLY proposals with status="APPLIED"
        """
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        applied = service.list_probes("APPLIED")

        assert len(applied) == 3
        assert all(p["status"] == "APPLIED" for p in applied)

    @pytest.mark.integration
    def test_list_filters_by_status_all(self, sample_proposals, self_prober):
        """
        Spec: list_probes("ALL") returns all proposals regardless of status
        """
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        all_proposals = service.list_probes("ALL")

        assert len(all_proposals) == 10


class TestFormalSpecGetProbe:
    """Formal Spec: Get single probe by ID."""

    @pytest.mark.integration
    def test_get_probe_found(self, sample_proposals, self_prober):
        """
        Spec: get_probe(probe_id) returns probe dict if found
        """
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        probe_id = sample_proposals[0].probe_id
        result = service.get_probe(probe_id)

        assert result is not None
        assert result["probe_id"] == probe_id
        assert "status" in result
        assert "dimension" in result

    @pytest.mark.integration
    def test_get_probe_not_found(self, self_prober):
        """
        Spec: get_probe(invalid_id) returns None
        """
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        result = service.get_probe("nonexistent_id")

        assert result is None


class TestFormalSpecStats:
    """Formal Spec: Statistics must be consistent."""

    @pytest.mark.integration
    def test_stats_counts_match_proposals(self, sample_proposals, self_prober):
        """
        Spec: stats() counts match actual proposal state.
        Verify: pending + applied + dismissed == queue_size
        """
        from cynic.kernel.organism.brain.cognition.cortex.probes_service import ProbesService
        service = ProbesService(self_prober, None)

        stats = service.get_stats()

        expected_size = 5 + 3 + 2  # pending + applied + dismissed
        assert stats["pending"] == 5
        assert stats["applied"] == 3
        assert stats["dismissed"] == 2
        assert stats["queue_size"] == expected_size
