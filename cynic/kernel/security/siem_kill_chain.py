"""
CYNIC Somatic SIEM - The Kill Chain Classifier.
Respects Security Architect & SRE Lenses.

Based on industry standard SIEM principles (Centralized Log Management, 
Event Correlation, and the 7-stage Kill Chain).
Transforms simple exceptions into actionable, prioritized defense states.
"""
from __future__ import annotations

from enum import Enum
from dataclasses import dataclass, field
import time

class KillChainStage(Enum):
    """The 7 stages of system compromise."""
    RECONNAISSANCE = 1       # E.g., unusual API probing, port scanning
    WEAPONIZATION = 2        # E.g., compiling malicious payload (in sandbox)
    DELIVERY = 3             # E.g., injecting corrupted data into VascularSystem
    EXPLOITATION = 4         # E.g., causing a localized crash or syntax error
    INSTALLATION = 5         # E.g., modifying core files (bypassing HeresyGuard)
    COMMAND_CONTROL = 6      # E.g., rogue LLM agent attempting to contact external IP
    ACTION_ON_OBJECTIVE = 7  # E.g., corrupting Q-Table, leaking Vault secrets

class SiemPriority(Enum):
    LOW = "LOW"
    MEDIUM = "MEDIUM"
    HIGH = "HIGH"
    CRITICAL = "CRITICAL"

@dataclass
class SecurityIncident:
    """A correlated SIEM event representing a threat to the Organism."""
    incident_id: str
    stage: KillChainStage
    priority: SiemPriority
    source: str
    description: str
    timestamp: float = field(default_factory=time.time)
    remediation_applied: bool = False

class SiemCorrelationEngine:
    """
    Analyzes streams of anomalies to detect Kill Chain progression.
    If an incident breaches Stage 4 (EXPLOITATION), priority escalates to HIGH.
    """
    
    @staticmethod
    def classify_anomaly(anomaly_type: str, context: str) -> SecurityIncident:
        """Rule-based classification of somatic anomalies."""
        
        # 1. Reconnaissance & Probing (Low Priority)
        if "timeout" in anomaly_type.lower() or "not found" in anomaly_type.lower():
            return SecurityIncident(
                incident_id=f"inc-{int(time.time())}",
                stage=KillChainStage.RECONNAISSANCE,
                priority=SiemPriority.LOW,
                source="perception",
                description=f"Probing detected: {context}"
            )
            
        # 2. Resource Exhaustion / Exploitation (High Priority)
        if "CPU_STRESS" in anomaly_type or "OOM" in anomaly_type:
            return SecurityIncident(
                incident_id=f"inc-{int(time.time())}",
                stage=KillChainStage.EXPLOITATION,
                priority=SiemPriority.HIGH,
                source="metabolism",
                description=f"Resource Exploitation (DoS attempt or runaway loop): {context}"
            )
            
        # 3. Core File Modification / Installation (Critical Priority)
        if "HERESY" in anomaly_type or "syntax" in context.lower():
            return SecurityIncident(
                incident_id=f"inc-{int(time.time())}",
                stage=KillChainStage.INSTALLATION,
                priority=SiemPriority.CRITICAL,
                source="cortex",
                description=f"Unauthorized core modification: {context}"
            )
            
        # Default to Delivery (Medium) if unknown
        return SecurityIncident(
            incident_id=f"inc-{int(time.time())}",
            stage=KillChainStage.DELIVERY,
            priority=SiemPriority.MEDIUM,
            source="unknown",
            description=f"Unclassified anomaly: {anomaly_type}"
        )
