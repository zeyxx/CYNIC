import sys
from src.pipeline.primitives import LogParser
from src.pipeline.heuristics import HeuristicAnalyzer
from src.pipeline.ml import MLAnalyzer
from src.pipeline.llm import LLMAnalyzer

def main():
    print("[Telemetry] Etape 1: Primitives (Extraction)")
    entries = list(LogParser.parse_antigravity_logs("~/.gemini/antigravity-cli/brain"))
    print(f"[Telemetry] {len(entries)} entrees brutes parsees.")
    
    print("[Telemetry] Etape 2: Heuristiques (Filtrage & Agregation)")
    heuristic = HeuristicAnalyzer()
    anomalies = heuristic.process_stream(entries)
    print(f"[Telemetry] {len(anomalies)} boucles d'anomalies detectees.")
    
    print("[Telemetry] Etape 3: Machine Learning (Clustering local)")
    ml = MLAnalyzer()
    clustered = ml.cluster_anomalies(anomalies)
    
    print("[Telemetry] Etape 4: LLM (Root Cause Analysis)")
    llm = LLMAnalyzer()
    report = llm.generate_rca(clustered)
    
    print("\n--- SYNTHESE RCA ---")
    print(report)

if __name__ == "__main__":
    main()
