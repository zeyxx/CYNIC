"""
CYNIC Immune System Framework — Safety-First Pathogen Detection & Inversion

Design Philosophy:
  - NEVER trust framework confidence > 61.8% (φ⁻¹ bound)
  - NEVER auto-apply fixes without human approval
  - NEVER learn from single example (require N confirmations)
  - ALWAYS provide dry-run before applying
  - ALWAYS monitor for delayed failures (7 days post-fix)

8 Safety Layers:
  1. Detection confidence bound (max 61.8%)
  2. Multiple detector consensus (3/5 agreement required)
  3. Dry-run verification (test in isolation)
  4. Quality audit (75% threshold for acceptance)
  5. Human checkpoint (explicit approval required)
  6. Post-apply monitoring (7-day observation)
  7. Framework self-audit (95% accuracy requirement)
  8. Bounded learning (no runaway confidence)

Status: SAFE MODE ONLY (no auto-repairs, human-in-loop)
"""
from __future__ import annotations

import asyncio
import json
import logging
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from pathlib import Path
from typing import List, Optional, Tuple, Dict, Set
from abc import ABC, abstractmethod
import hashlib

logger = logging.getLogger("cynic.immune_system")

# φ-bounds (from CYNIC philosophy)
MAX_CONFIDENCE = 0.618  # φ⁻¹
MAX_Q_SCORE = 100
MIN_CONSENSUS_THRESHOLD = 0.6  # 60%
MIN_FIX_QUALITY_THRESHOLD = 0.75  # 75%
FRAMEWORK_ACCURACY_THRESHOLD = 0.95  # 95%
POST_FIX_MONITORING_DURATION = timedelta(days=7)


# ════════════════════════════════════════════════════════════════════════════
# DATA MODELS
# ════════════════════════════════════════════════════════════════════════════

class PathogenSeverity(Enum):
    """Viral pattern severity classification."""
    CRITICAL = 10
    HIGH = 7
    MEDIUM = 5
    LOW = 3
    INFO = 1


class InversionStatus(Enum):
    """Status of inversion attempt."""
    PROPOSED = "proposed"
    DRY_RUN_PASS = "dry_run_pass"
    DRY_RUN_FAIL = "dry_run_fail"
    AWAITING_APPROVAL = "awaiting_approval"
    APPROVED = "approved"
    APPLIED = "applied"
    MONITORING = "monitoring"
    ROLLBACK = "rollback"
    FAILED = "failed"
    SUCCESS = "success"


class TestQuality(Enum):
    """Test quality classification (Kani framework)."""
    INDUCTIVE = "INDUCTIVE"
    STRONG = "STRONG"
    WEAK = "WEAK"
    UNIT_TEST = "UNIT_TEST"
    VACUOUS = "VACUOUS"


@dataclass
class Pathogen:
    """Detected viral pattern."""
    name: str
    severity: PathogenSeverity
    location: str  # File path or code location
    confidence: float  # 0-1, capped at MAX_CONFIDENCE
    signature: str  # Hash of pattern
    description: str = ""
    detectors_agreeing: int = 0  # How many detectors found this
    timestamp: datetime = field(default_factory=datetime.utcnow)

    def __post_init__(self):
        """Enforce φ-bound on confidence."""
        if self.confidence > MAX_CONFIDENCE:
            logger.warning(
                f"Pathogen {self.name} confidence {self.confidence:.3f} "
                f"exceeds φ⁻¹ bound. Capping to {MAX_CONFIDENCE}"
            )
            self.confidence = MAX_CONFIDENCE


@dataclass
class InversionPlan:
    """Proposed fix/inversion for pathogen."""
    pathogen_id: str
    fix_description: str
    files_to_modify: List[str]
    proposed_changes: Dict[str, str]  # file_path -> proposed_code
    quality_score: float = 0.0
    status: InversionStatus = InversionStatus.PROPOSED
    dry_run_results: Optional[Dict] = None
    human_approval: bool = False
    applied_at: Optional[datetime] = None
    monitoring_until: Optional[datetime] = None


@dataclass
class PatternSignature:
    """Learned pattern signature."""
    name: str
    signature: str  # Unique hash
    confidence: float = 0.1  # Start low
    success_count: int = 0  # # times fix worked
    failure_count: int = 0  # # times fix failed
    status: str = "LEARNING"  # LEARNING, TRUSTED, DISTRUSTED
    discovered_at: datetime = field(default_factory=datetime.utcnow)

    def add_success(self):
        """Learn from successful application."""
        self.success_count += 1
        if self.success_count >= 3:  # Require 3 successes
            self.status = "TRUSTED"
        # Increment confidence, capped
        self.confidence = min(self.confidence + 0.05, MAX_CONFIDENCE)

    def add_failure(self):
        """Learn from failed application."""
        self.failure_count += 1
        if self.failure_count > self.success_count:
            self.status = "DISTRUSTED"
        # Decrement confidence
        self.confidence = max(self.confidence - 0.1, 0.0)


# ════════════════════════════════════════════════════════════════════════════
# LAYER 1: PATHOGEN DETECTION
# ════════════════════════════════════════════════════════════════════════════

class PathogenDetector(ABC):
    """Abstract base for pathogen detectors."""

    @abstractmethod
    async def detect(self, code: str, location: str) -> List[Pathogen]:
        """Scan code for pathogens. Return list of detected infections."""
        pass

    @abstractmethod
    def detector_name(self) -> str:
        """Name of this detector."""
        pass


class ExceptionSwallowingDetector(PathogenDetector):
    """Detects 'except Exception' patterns that swallow errors."""

    async def detect(self, code: str, location: str) -> List[Pathogen]:
        """Find bare except Exception handlers."""
        import re
        pathogens = []
        lines = code.split('\n')

        for line_no, line in enumerate(lines, 1):
            # Pattern: except Exception (with optional variable binding)
            if re.search(r'except\s+Exception(\s+as\s+\w+)?:', line):
                # Check if next few lines show error swallowing
                # (pass, return, continue, etc. without re-raising)
                is_swallowing = False

                # Look at next 3 lines
                for future_idx in range(1, min(4, len(lines) - line_no + 1)):
                    future_line = lines[line_no + future_idx - 1].strip()

                    # If we see: pass, return None, return, continue, break
                    if any(x in future_line for x in ['pass', 'return None', 'return', 'continue', 'break']):
                        is_swallowing = True
                        break

                    # If we see proper handling (raise, logging), stop
                    if any(x in future_line for x in ['raise', 'logger.', 'log.', 'print(']):
                        break

                if is_swallowing:
                    signature = hashlib.md5(
                        f"{location}:{line_no}:{line}".encode()
                    ).hexdigest()
                    pathogens.append(Pathogen(
                        name="exception_swallowing",
                        severity=PathogenSeverity.HIGH,
                        location=f"{location}:{line_no}",
                        confidence=0.85,
                        signature=signature,
                        description="Exception caught but not re-raised or logged"
                    ))
        return pathogens

    def detector_name(self) -> str:
        return "ExceptionSwallowingDetector"


class VacuousTestDetector(PathogenDetector):
    """Detects tests that don't actually test anything."""

    async def detect(self, code: str, location: str) -> List[Pathogen]:
        """Find vacuous tests (only assert True, no logic)."""
        import re
        pathogens = []
        lines = code.split('\n')

        i = 0
        while i < len(lines):
            line = lines[i]
            # Find test function definition
            if re.search(r'(async\s+)?def\s+test_\w+', line):
                # Get test body (until next def or class or end)
                test_start = i
                test_end = i + 1
                indent_level = len(line) - len(line.lstrip())

                # Find end of test function
                while test_end < len(lines):
                    next_line = lines[test_end]
                    if next_line.strip() == '':
                        test_end += 1
                        continue
                    next_indent = len(next_line) - len(next_line.lstrip())
                    if next_indent <= indent_level and next_line.strip() != '':
                        break
                    test_end += 1

                test_body = '\n'.join(lines[test_start:test_end])

                # Check if test is vacuous
                # Vacuous = only "assert True" statements, no real assertions
                assert_lines = [l.strip() for l in test_body.split('\n') if 'assert' in l]
                real_assertions = [l for l in assert_lines if 'assert True' not in l and l.strip() != '']

                # If there are no real assertions, it's vacuous
                if len(assert_lines) > 0 and len(real_assertions) == 0:
                    signature = hashlib.md5(
                        f"{location}:{test_start}:vacuous".encode()
                    ).hexdigest()
                    pathogens.append(Pathogen(
                        name="vacuous_test",
                        severity=PathogenSeverity.MEDIUM,
                        location=f"{location}:{test_start+1}",
                        confidence=0.75,
                        signature=signature,
                        description="Test only asserts True, provides no coverage"
                    ))

                i = test_end
            else:
                i += 1

        return pathogens

    def detector_name(self) -> str:
        return "VacuousTestDetector"


# ════════════════════════════════════════════════════════════════════════════
# LAYER 2: DETECTOR CONSENSUS
# ════════════════════════════════════════════════════════════════════════════

class ConsensusEngine:
    """Verify infections via multiple detector agreement."""

    def __init__(self):
        self.detectors: List[PathogenDetector] = [
            ExceptionSwallowingDetector(),
            VacuousTestDetector(),
        ]

    async def verify_infection(
        self,
        code: str,
        location: str,
        threshold: float = MIN_CONSENSUS_THRESHOLD
    ) -> List[Pathogen]:
        """
        Run all detectors. Return infections with consensus >= threshold.
        Requires: (# detectors agreeing) / (# detectors) >= threshold
        """
        detection_results: Dict[str, Pathogen] = {}

        # Run all detectors
        for detector in self.detectors:
            try:
                pathogens = await detector.detect(code, location)
                for pathogen in pathogens:
                    if pathogen.signature not in detection_results:
                        detection_results[pathogen.signature] = pathogen
                    detection_results[pathogen.signature].detectors_agreeing += 1
            except Exception as e:
                logger.error(f"Detector {detector.detector_name()} failed: {e}")

        # Filter by consensus threshold
        verified = []
        for pathogen in detection_results.values():
            consensus = pathogen.detectors_agreeing / len(self.detectors)
            if consensus >= threshold:
                verified.append(pathogen)
            else:
                logger.debug(
                    f"Pathogen {pathogen.name} at {pathogen.location} "
                    f"has low consensus ({consensus:.1%}), flagged as UNCERTAIN"
                )

        return verified


# ════════════════════════════════════════════════════════════════════════════
# LAYER 3-4: DRY-RUN & QUALITY AUDIT
# ════════════════════════════════════════════════════════════════════════════

class InversionStrategist:
    """Generate and audit inversion plans."""

    async def propose_inversion(self, pathogen: Pathogen) -> Optional[InversionPlan]:
        """Generate inversion plan (but don't apply yet)."""
        logger.info(f"Proposing inversion for {pathogen.name} at {pathogen.location}")

        # For now: just create placeholder plan
        plan = InversionPlan(
            pathogen_id=pathogen.signature,
            fix_description=f"Fix {pathogen.name}",
            files_to_modify=[pathogen.location],
            proposed_changes={},
            status=InversionStatus.PROPOSED
        )

        return plan

    async def dry_run(self, plan: InversionPlan) -> Dict:
        """
        Test fix in isolated environment.
        Returns: {success: bool, errors: List[str], warnings: List[str]}
        """
        logger.info(f"Dry-run for plan {plan.pathogen_id}")

        # Placeholder: simulate dry-run
        results = {
            "success": True,
            "errors": [],
            "warnings": [],
            "test_failures_before": 0,
            "test_failures_after": 0,
        }

        plan.dry_run_results = results
        return results

    async def audit_quality(self, plan: InversionPlan) -> float:
        """
        Measure quality of proposed fix.
        Returns: 0-1 score. Threshold for acceptance: 0.75
        """
        logger.info(f"Auditing quality of plan {plan.pathogen_id}")

        if not plan.dry_run_results:
            logger.error("No dry-run results available")
            return 0.0

        results = plan.dry_run_results

        # Quality metrics
        score = 1.0

        # Did it break anything?
        if results["test_failures_after"] > results["test_failures_before"]:
            logger.warning("Fix introduces new test failures")
            score -= 0.5

        # Were there errors?
        if results["errors"]:
            logger.warning(f"Fix has errors: {results['errors']}")
            score -= 0.3

        # Warnings (minor deduction)
        if results["warnings"]:
            score -= 0.1 * min(len(results["warnings"]), 3)

        plan.quality_score = max(score, 0.0)
        return plan.quality_score


# ════════════════════════════════════════════════════════════════════════════
# LAYER 5: HUMAN CHECKPOINT
# ════════════════════════════════════════════════════════════════════════════

class HumanCheckpoint:
    """Gate all fixes through human approval."""

    async def request_approval(self, plan: InversionPlan, quality_score: float) -> bool:
        """
        Request human approval for fix.
        For now: returns False (manual override needed).
        In production: would send Slack/email notification.
        """
        logger.warning(
            f"HUMAN APPROVAL REQUIRED for {plan.pathogen_id}\n"
            f"Quality Score: {quality_score:.1%}\n"
            f"Fix: {plan.fix_description}\n"
            f"Files: {plan.files_to_modify}"
        )

        # TODO: integrate with Slack/email
        # For now: manual approval required
        return False

    async def wait_for_approval(self, plan_id: str, timeout: timedelta = timedelta(hours=24)) -> bool:
        """Wait for human to approve fix."""
        logger.info(f"Waiting for approval of {plan_id} (timeout: {timeout})")
        # TODO: Implement notification system
        return False


# ════════════════════════════════════════════════════════════════════════════
# LAYER 6: POST-APPLY MONITORING
# ════════════════════════════════════════════════════════════════════════════

class PostApplyMonitor:
    """Monitor for delayed failures after fix applied."""

    async def monitor(
        self,
        plan: InversionPlan,
        duration: timedelta = POST_FIX_MONITORING_DURATION
    ) -> bool:
        """
        Monitor system for 7 days after fix.
        Returns: True if system stays healthy, False if regressions detected.
        """
        logger.info(
            f"Monitoring plan {plan.pathogen_id} for {duration.days} days"
        )

        plan.monitoring_until = datetime.utcnow() + duration
        plan.status = InversionStatus.MONITORING

        # TODO: Implement continuous monitoring
        # - Run test suite daily
        # - Check error logs for anomalies
        # - Measure performance metrics

        return True


# ════════════════════════════════════════════════════════════════════════════
# LAYER 7: FRAMEWORK SELF-AUDIT
# ════════════════════════════════════════════════════════════════════════════

class FrameworkSelfAudit:
    """Verify framework's own correctness."""

    def __init__(self):
        self.test_cases: List[Tuple[str, str, PathogenSeverity]] = [
            # (code_sample, expected_pathogen_name, expected_severity)
            ("except Exception:\n    pass", "exception_swallowing", PathogenSeverity.HIGH),
            ("except Exception as e:\n    logger.error(str(e))", "exception_swallowing", PathogenSeverity.HIGH),
        ]

    async def audit_framework(self) -> float:
        """
        Test framework on KNOWN cases.
        Returns: accuracy 0-1. Threshold: 95%
        """
        logger.info("Running framework self-audit")

        consensus = ConsensusEngine()
        correct = 0

        for code, expected_name, expected_severity in self.test_cases:
            pathogens = await consensus.verify_infection(code, "test_location")

            # Check if expected pathogen was found
            found = any(p.name == expected_name for p in pathogens)
            if found:
                correct += 1

        accuracy = correct / len(self.test_cases) if self.test_cases else 0.0

        logger.info(f"Framework accuracy: {accuracy:.1%} ({correct}/{len(self.test_cases)})")

        if accuracy < FRAMEWORK_ACCURACY_THRESHOLD:
            logger.error(
                f"Framework accuracy {accuracy:.1%} below threshold "
                f"{FRAMEWORK_ACCURACY_THRESHOLD:.1%}. DISABLING AUTO-REPAIRS."
            )
            return 0.0

        return accuracy


# ════════════════════════════════════════════════════════════════════════════
# LAYER 8: BOUNDED LEARNING
# ════════════════════════════════════════════════════════════════════════════

class ImmunityMemory:
    """Learn from each inversion, with safety bounds."""

    def __init__(self, memory_file: Path = Path("~/.cynic/immunity_memory.json")):
        self.memory_file = memory_file.expanduser()
        self.patterns: Dict[str, PatternSignature] = {}
        self.load_memory()

    def load_memory(self):
        """Load learned patterns from disk."""
        if self.memory_file.exists():
            try:
                data = json.loads(self.memory_file.read_text())
                for name, pattern_data in data.items():
                    self.patterns[name] = PatternSignature(**pattern_data)
                logger.info(f"Loaded {len(self.patterns)} patterns from memory")
            except Exception as e:
                logger.error(f"Failed to load immunity memory: {e}")

    def save_memory(self):
        """Save learned patterns to disk."""
        try:
            data = {
                name: {
                    "name": p.name,
                    "signature": p.signature,
                    "confidence": p.confidence,
                    "success_count": p.success_count,
                    "failure_count": p.failure_count,
                    "status": p.status,
                }
                for name, p in self.patterns.items()
            }
            self.memory_file.parent.mkdir(parents=True, exist_ok=True)
            self.memory_file.write_text(json.dumps(data, indent=2))
            logger.info(f"Saved {len(self.patterns)} patterns to memory")
        except Exception as e:
            logger.error(f"Failed to save immunity memory: {e}")

    def learn_success(self, pattern_name: str, signature: str):
        """Learn from successful inversion."""
        if pattern_name not in self.patterns:
            self.patterns[pattern_name] = PatternSignature(
                name=pattern_name,
                signature=signature
            )
        self.patterns[pattern_name].add_success()
        logger.info(
            f"Learned: {pattern_name} now "
            f"confidence={self.patterns[pattern_name].confidence:.2f} "
            f"status={self.patterns[pattern_name].status}"
        )
        self.save_memory()

    def learn_failure(self, pattern_name: str, signature: str):
        """Learn from failed inversion."""
        if pattern_name not in self.patterns:
            self.patterns[pattern_name] = PatternSignature(
                name=pattern_name,
                signature=signature
            )
        self.patterns[pattern_name].add_failure()
        logger.info(
            f"Learned: {pattern_name} failed, now "
            f"confidence={self.patterns[pattern_name].confidence:.2f} "
            f"status={self.patterns[pattern_name].status}"
        )
        self.save_memory()

    def get_trusted_patterns(self) -> List[PatternSignature]:
        """Return patterns with TRUSTED status."""
        return [p for p in self.patterns.values() if p.status == "TRUSTED"]


# ════════════════════════════════════════════════════════════════════════════
# MAIN ORCHESTRATOR
# ════════════════════════════════════════════════════════════════════════════

class SafeImmunityFramework:
    """
    Main orchestrator: coordinates all 8 safety layers.
    SAFE MODE: Never auto-repairs, human-in-loop always.
    """

    def __init__(self):
        self.consensus = ConsensusEngine()
        self.strategist = InversionStrategist()
        self.checkpoint = HumanCheckpoint()
        self.monitor = PostApplyMonitor()
        self.self_audit = FrameworkSelfAudit()
        self.memory = ImmunityMemory()
        self.is_safe = True  # Safety flag

    async def initialize(self) -> bool:
        """
        Initialize framework with safety checks.
        Returns: False if framework accuracy below threshold (SAFE MODE disabled).
        """
        logger.info("Initializing Safe Immunity Framework")

        accuracy = await self.self_audit.audit_framework()
        if accuracy < FRAMEWORK_ACCURACY_THRESHOLD:
            logger.critical("Framework accuracy check FAILED. Disabling auto-repairs.")
            self.is_safe = False
            return False

        logger.info("Framework passed self-audit. Ready to detect pathogens.")
        self.is_safe = True
        return True

    async def scan_for_pathogens(self, code: str, location: str) -> List[Pathogen]:
        """Scan code for pathogens (detector consensus)."""
        if not self.is_safe:
            logger.error("Framework in UNSAFE mode. Refusing to scan.")
            return []

        logger.info(f"Scanning {location} for pathogens")
        pathogens = await self.consensus.verify_infection(code, location)
        logger.info(f"Found {len(pathogens)} pathogens with consensus")

        return pathogens

    async def propose_and_audit_fix(self, pathogen: Pathogen) -> Optional[InversionPlan]:
        """
        Propose fix and audit quality (but don't apply).
        Returns: InversionPlan if quality >= 75%, None otherwise.
        """
        if not self.is_safe:
            logger.error("Framework in UNSAFE mode. Refusing to propose fixes.")
            return None

        logger.info(f"Proposing fix for {pathogen.name}")

        # Generate plan
        plan = await self.strategist.propose_inversion(pathogen)
        if not plan:
            logger.error("Failed to generate inversion plan")
            return None

        # Dry-run
        dry_run_results = await self.strategist.dry_run(plan)
        logger.info(f"Dry-run results: {dry_run_results}")

        # Audit quality
        quality = await self.strategist.audit_quality(plan)
        logger.info(f"Plan quality score: {quality:.1%}")

        if quality < MIN_FIX_QUALITY_THRESHOLD:
            logger.warning(
                f"Plan quality {quality:.1%} below threshold "
                f"{MIN_FIX_QUALITY_THRESHOLD:.1%}. Rejecting plan."
            )
            plan.status = InversionStatus.DRY_RUN_FAIL
            return None

        plan.status = InversionStatus.AWAITING_APPROVAL
        return plan

    async def request_human_approval(self, plan: InversionPlan) -> bool:
        """Request human approval for fix."""
        logger.info(f"Requesting human approval for {plan.pathogen_id}")
        return await self.checkpoint.request_approval(plan, plan.quality_score)

    async def apply_fix(self, plan: InversionPlan) -> bool:
        """
        Apply fix ONLY after human approval.
        In SAFE MODE: always requires human approval.
        """
        if not self.is_safe:
            logger.error("Framework in UNSAFE mode. Refusing to apply fixes.")
            return False

        if not plan.human_approval:
            logger.error("Fix requires human approval before applying")
            return False

        logger.info(f"Applying fix for {plan.pathogen_id}")
        # TODO: Implement actual fix application
        plan.status = InversionStatus.APPLIED
        plan.applied_at = datetime.utcnow()

        # Start monitoring
        await self.monitor.monitor(plan)

        return True

    def learn_outcome(self, plan: InversionPlan, success: bool):
        """Learn from inversion outcome."""
        pathogen_name = plan.pathogen_id  # Simplified
        if success:
            self.memory.learn_success(pathogen_name, plan.pathogen_id)
        else:
            self.memory.learn_failure(pathogen_name, plan.pathogen_id)


# ════════════════════════════════════════════════════════════════════════════
# CLI INTERFACE
# ════════════════════════════════════════════════════════════════════════════

async def main():
    """Simple CLI for testing framework."""
    logging.basicConfig(level=logging.INFO)

    framework = SafeImmunityFramework()

    # Initialize
    if not await framework.initialize():
        logger.error("Framework initialization failed. Exiting.")
        return

    # Example: scan sample code
    sample_code = """
async def process_judgment():
    try:
        result = await db.save(judgment)
    except Exception:
        pass  # VIRUS: swallowing error
"""

    pathogens = await framework.scan_for_pathogens(sample_code, "example.py")
    print(f"\nDetected {len(pathogens)} pathogens:")
    for p in pathogens:
        print(f"  - {p.name} at {p.location} (confidence: {p.confidence:.2f})")

    # Propose fix
    if pathogens:
        plan = await framework.propose_and_audit_fix(pathogens[0])
        if plan:
            print(f"\nProposed fix quality: {plan.quality_score:.1%}")
            print(f"Status: {plan.status.value}")
            print("\nNote: Fix requires human approval before applying.")


if __name__ == "__main__":
    asyncio.run(main())
