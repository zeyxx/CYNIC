"""
Tests for PowerLimiter guardrail.
"""
import pytest
import time
import logging
from unittest.mock import MagicMock, patch
from cynic.immune.power_limiter import PowerLimiter, ResourceMetrics, HAS_PSUTIL
from cynic.core.consciousness import ConsciousnessLevel


class TestPowerLimiterInit:
    def test_init_default_state(self):
        limiter = PowerLimiter()
        assert limiter._judgment_timestamps == []
        assert limiter._action_timestamps == []
        assert limiter._warned_cpu is False
        assert limiter._warned_memory is False

    def test_start_logging(self, caplog):
        limiter = PowerLimiter()
        with caplog.at_level(logging.INFO):
            limiter.start()
        assert "PowerLimiter started" in caplog.text


class TestPowerLimiterMetrics:
    def test_record_judgment(self):
        limiter = PowerLimiter()
        limiter.record_judgment()
        limiter.record_judgment()
        limiter.record_judgment()
        assert len(limiter._judgment_timestamps) == 3

    def test_record_action(self):
        limiter = PowerLimiter()
        limiter.record_action()
        limiter.record_action()
        assert len(limiter._action_timestamps) == 2

    def test_judgment_window_cleanup(self):
        """Old judgment records should be cleaned after 1s window."""
        limiter = PowerLimiter()
        # Add old timestamp
        old_time = time.time() - 2.0
        limiter._judgment_timestamps.append(old_time)
        limiter._judgment_timestamps.append(time.time())

        # Trigger cleanup by recording new judgment
        limiter.record_judgment()

        # Old entry should be gone
        assert len(limiter._judgment_timestamps) == 2  # new one + recent one
        assert all(ts > time.time() - 1.1 for ts in limiter._judgment_timestamps)

    def test_action_window_cleanup(self):
        """Old action records should be cleaned after 1m window."""
        limiter = PowerLimiter()
        # Add old timestamp (61s ago)
        old_time = time.time() - 61.0
        limiter._action_timestamps.append(old_time)
        limiter._action_timestamps.append(time.time())

        # Trigger cleanup
        limiter.record_action()

        # Old entry should be gone
        assert len(limiter._action_timestamps) == 2
        assert all(ts > time.time() - 60.1 for ts in limiter._action_timestamps)


class TestPowerLimiterHealthChecks:
    def _make_scheduler(self, task_count=0, queue_depth=0):
        """Create mock scheduler."""
        scheduler = MagicMock()
        scheduler._tasks = [MagicMock()] * task_count

        # Mock queues
        queue1 = MagicMock()
        queue1.qsize.return_value = queue_depth // 2 if queue_depth > 0 else 0
        queue2 = MagicMock()
        queue2.qsize.return_value = queue_depth // 2 if queue_depth > 0 else 0
        scheduler._queues = {"REFLEX": queue1, "MICRO": queue2}

        return scheduler

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_can_accept_work_healthy_system(self):
        """Healthy system should accept work."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler(task_count=2, queue_depth=5)

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 30.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            assert limiter.check_available(scheduler) is True

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_reject_work_high_cpu(self):
        """Should reject work when CPU > 80%."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 85.0
            mock_proc_instance.memory_percent.return_value = 50.0
            mock_psutil.Process.return_value = mock_proc_instance

            assert limiter.check_available(scheduler) is False

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_reject_work_high_memory(self):
        """Should reject work when memory > 85%."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 50.0
            mock_proc_instance.memory_percent.return_value = 90.0
            mock_psutil.Process.return_value = mock_proc_instance

            assert limiter.check_available(scheduler) is False

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_reject_work_judgment_rate_limit(self):
        """Should reject work when judgments exceed rate limit."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        # Fill judgment buffer beyond limit (5/sec)
        for _ in range(6):
            limiter.record_judgment()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 30.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            assert limiter.check_available(scheduler) is False

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_reject_work_action_rate_limit(self):
        """Should reject work when actions exceed rate limit."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        # Fill action buffer beyond limit (13/min)
        for _ in range(14):
            limiter.record_action()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 30.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            assert limiter.check_available(scheduler) is False


class TestPowerLimiterRecommendedLevel:
    def _make_scheduler(self, task_count=0, queue_depth=0):
        scheduler = MagicMock()
        scheduler._tasks = [MagicMock()] * task_count
        queue1 = MagicMock()
        queue1.qsize.return_value = queue_depth // 2 if queue_depth > 0 else 0
        queue2 = MagicMock()
        queue2.qsize.return_value = queue_depth // 2 if queue_depth > 0 else 0
        scheduler._queues = {"REFLEX": queue1, "MICRO": queue2}
        return scheduler

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_recommend_macro_healthy(self):
        """Healthy system should allow MACRO."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 30.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            level = limiter.recommended_level(scheduler)
            assert level == ConsciousnessLevel.MACRO

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_recommend_micro_cpu_pressure(self):
        """70% CPU should recommend MICRO."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 75.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            level = limiter.recommended_level(scheduler)
            assert level == ConsciousnessLevel.MICRO

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_recommend_reflex_memory_critical(self):
        """High memory should recommend REFLEX."""
        limiter = PowerLimiter()
        scheduler = self._make_scheduler()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 50.0
            mock_proc_instance.memory_percent.return_value = 90.0
            mock_psutil.Process.return_value = mock_proc_instance

            level = limiter.recommended_level(scheduler)
            assert level == ConsciousnessLevel.REFLEX

    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_recommend_reflex_queue_critical(self):
        """High queue depth should recommend REFLEX."""
        limiter = PowerLimiter()
        # queue_depth = 22 (> F(8)=21 threshold)
        scheduler = self._make_scheduler(queue_depth=22)

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 30.0
            mock_proc_instance.memory_percent.return_value = 40.0
            mock_psutil.Process.return_value = mock_proc_instance

            level = limiter.recommended_level(scheduler)
            assert level == ConsciousnessLevel.REFLEX


class TestPowerLimiterStats:
    @pytest.mark.skipif(not HAS_PSUTIL, reason="psutil not installed")
    def test_stats_format(self):
        limiter = PowerLimiter()
        limiter.record_judgment()
        limiter.record_action()

        with patch('cynic.immune.power_limiter.psutil') as mock_psutil:
            mock_proc_instance = MagicMock()
            mock_proc_instance.cpu_percent.return_value = 45.0
            mock_proc_instance.memory_percent.return_value = 55.0
            mock_psutil.Process.return_value = mock_proc_instance

            stats = limiter.stats()

        assert "cpu_pct" in stats
        assert "memory_pct" in stats
        assert "judgments_per_sec" in stats
        assert "actions_per_min" in stats
        assert stats["cpu_pct"] == 45.0
        assert stats["memory_pct"] == 55.0
        assert stats["judgments_per_sec"] >= 1
        assert stats["actions_per_min"] >= 1
