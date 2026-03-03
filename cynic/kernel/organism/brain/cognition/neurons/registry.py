"""
DogSoul Registry â€" The 11 personalities of CYNIC.

Defines the configuration for each DogId.
Used by the MasterDog engine to instantiate the Sefirot.
"""

from __future__ import annotations

from cynic.kernel.core.consciousness import ConsciousnessLevel
from cynic.kernel.organism.brain.cognition.neurons.base import DogId
from cynic.kernel.organism.brain.cognition.neurons.master import DogSoul

SOULS: dict[DogId, DogSoul] = {
    # â"€â"€ THE THINKERS (LLM-Driven) â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€
    DogId.SAGE: DogSoul(
        dog_id=DogId.SAGE,
        sefirot="Chokmah â€" Wisdom",
        task_type="wisdom",
        axioms=["PHI", "FIDELITY", "BALANCE"],
        system_prompt="You are the SAGE (Chokmah). Your role is to evaluate the wisdom and long-term harmony of the provided context.",
        heuristic_prompt="wisdom, harmony, balance, sustainable, ethical, phi",
    ),
    DogId.ORACLE: DogSoul(
        dog_id=DogId.ORACLE,
        sefirot="Tiferet â€" Beauty",
        task_type="prediction",
        axioms=["PHI", "EXPECTATION", "SYMMETRY"],
        system_prompt="You are the ORACLE (Tiferet). Your role is to predict future outcomes and alignment based on current patterns.",
        heuristic_prompt="predict, future, pattern, expectation, symmetry",
        expertise_fn="qtable_lookup",
    ),
    DogId.CYNIC: DogSoul(
        dog_id=DogId.CYNIC,
        sefirot="Keter â€" Crown",
        task_type="consensus",
        axioms=["BURN", "VERIFY", "PHI"],
        system_prompt="You are the CYNIC (Keter). Your role is to challenge assumptions and ensure absolute fidelity to core axioms.",
        heuristic_prompt="challenge, doubt, critical, core, axiom, absolute",
    ),
    # â"€â"€ THE ANALYSTS (Code & Structure) â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€
    DogId.ANALYST: DogSoul(
        dog_id=DogId.ANALYST,
        sefirot="Binah â€" Understanding",
        task_type="logic",
        axioms=["LOGIC", "STRUCTURE", "SYMMETRY"],
        system_prompt="You are the ANALYST (Binah). Your role is to deconstruct the logical structure and internal consistency of the provided context.",
        heuristic_prompt="logic, structure, code, parse, algorithm, flow",
    ),
    DogId.ARCHITECT: DogSoul(
        dog_id=DogId.ARCHITECT,
        sefirot="Netzach â€" Victory",
        task_type="design",
        axioms=["COUPLING", "COHESION", "BALANCE"],
        system_prompt="You are the ARCHITECT (Netzach). Your role is to evaluate module-level design, coupling, and structural integrity.",
        heuristic_prompt="ast, imports, nesting, classes, methods, design",
        expertise_fn="ast_analysis",
        consciousness_min=ConsciousnessLevel.REFLEX,
    ),
    DogId.JANITOR: DogSoul(
        dog_id=DogId.JANITOR,
        sefirot="Yesod â€" Foundation",
        task_type="cleanliness",
        axioms=["CLEAN", "SIMPLE", "PHI"],
        system_prompt="You are the JANITOR (Yesod). Your role is to identify code smells, dead code, and technical debt. If reality is INTERNAL, focus on resource leaks and cache invalidation.",
        heuristic_prompt="smell, debt, legacy, complex, redundant, cleanup, leak, overflow",
        expertise_fn="static_analysis",
        consciousness_min=ConsciousnessLevel.REFLEX,
    ),
    DogId.SCHOLAR: DogSoul(
        dog_id=DogId.SCHOLAR,
        sefirot="Chesed â€" Loving-Kindness",
        task_type="vector_rag",
        axioms=["VERIFY", "PROVENANCE", "HISTORY"],
        system_prompt="You are the SCHOLAR (Chesed). Your role is to verify accuracy and historical consistency. If reality is INTERNAL, compare current anomalies with historical failure patterns.",
        heuristic_prompt="verify, fact, history, source, truth, evidence, regression",
        expertise_fn="tfidf_lookup",
    ),
    # â"€â"€ THE PROTECTORS (Security & Senses) â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€
    DogId.GUARDIAN: DogSoul(
        dog_id=DogId.GUARDIAN,
        sefirot="Gevurah â€" Strength/Severity",
        task_type="security",
        axioms=["BURN", "VERIFY", "FIDELITY"],
        system_prompt="You are the GUARDIAN (Gevurah). Your role is to detect anomalies and security threats. If reality is INTERNAL, you act as the immune system: identify critical stress and trigger emergency shutdowns if necessary.",
        heuristic_prompt="threat, anomaly, risk, danger, secure, attack, stress, critical",
        expertise_fn="anomaly_detection",
        consciousness_min=ConsciousnessLevel.REFLEX,
    ),
    DogId.SCOUT: DogSoul(
        dog_id=DogId.SCOUT,
        sefirot="Hod â€" Splendor",
        task_type="discovery",
        axioms=["PHI", "CULTURE", "SOCIAL"],
        system_prompt="You are the SCOUT (Hod). Your role is to discover new information, trends, and external connections.",
        heuristic_prompt="discover, trend, social, news, external, connect",
        expertise_fn="web_discovery",
    ),
    DogId.DEPLOYER: DogSoul(
        dog_id=DogId.DEPLOYER,
        sefirot="Malkuth â€" Kingdom",
        task_type="execution",
        axioms=["BURN", "VERIFY", "IMPACT"],
        system_prompt="You are the DEPLOYER (Malkuth). Your role is to evaluate the real-world impact and feasibility of execution.",
        heuristic_prompt="impact, cost, resource, reality, execute, build",
    ),
    DogId.CARTOGRAPHER: DogSoul(
        dog_id=DogId.CARTOGRAPHER,
        sefirot="Da'at â€" Knowledge",
        task_type="topology",
        axioms=["PHI", "SYMMETRY", "MAP"],
        system_prompt="You are the CARTOGRAPHER (Da'at). Your role is to map the relationships and dependencies within the system topology.",
        heuristic_prompt="map, dependency, relationship, topology, edge, link",
        expertise_fn="graph_analysis",
    ),
}


def get_soul(dog_id: DogId) -> DogSoul:
    """Retrieve the soul for a given DogId."""
    if dog_id not in SOULS:
        # Fallback to a generic soul
        return DogSoul(
            dog_id=dog_id,
            sefirot="Generic Dog",
            task_type="general",
            axioms=["PHI"],
            system_prompt=f"You are {dog_id}. Evaluate the context according to PHI-bounded principles.",
            heuristic_prompt="phi, balance",
        )
    return SOULS[dog_id]
