from __future__ import annotations

import os

from src.adapters.surreal_adapter import SurrealTelemetryAdapter
from src.pipeline.heuristics import HeuristicAnalyzer
from src.pipeline.llm import LLMAnalyzer
from src.pipeline.ml import MLAnalyzer
from src.pipeline.primitives import LogParser


def main() -> int:
    print("[Telemetry] Etape 1: Primitives (Extraction)")
    entries = list(LogParser.parse_antigravity_logs("~/.gemini/antigravity-cli/brain"))
    print(f"[Telemetry] {len(entries)} entrees brutes parsees.")

    print("[Telemetry] Etape 2: Heuristiques (Filtrage & Agregation)")
    heuristic = HeuristicAnalyzer()
    anomalies = heuristic.process_stream(entries)
    print(f"[Telemetry] {len(anomalies)} boucles d'anomalies detectees.")

    surreal_url = os.environ.get("SURREALDB_URL", "http://localhost:8000")
    surreal_credential = os.environ.get("SURREALDB_PASS", "root")
    print(f"[Telemetry] Envoi de {len(anomalies)} anomalies vers SurrealDB ({surreal_url})...")
    adapter = SurrealTelemetryAdapter(
        url=surreal_url,
        ns="cynic",
        db="main",
        user="root",
        credential=surreal_credential,
    )
    success = adapter.store_anomalies(anomalies)
    if success:
        print("[Telemetry] Anomalies stockees avec succes.")
    else:
        print("[Telemetry] Echec du stockage des anomalies.")

    print("[Telemetry] Etape 3: Machine Learning (Clustering local)")
    ml = MLAnalyzer()
    clustered = ml.cluster_anomalies(anomalies)

    print("[Telemetry] Etape 4: LLM (Root Cause Analysis)")
    llm = LLMAnalyzer()
    report = llm.generate_rca(clustered)

    print("\n--- SYNTHESE RCA ---")
    print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
