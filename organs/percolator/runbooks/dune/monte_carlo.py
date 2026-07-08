import random
import csv
import statistics
from pathlib import Path

ROOT = next(path for path in Path(__file__).resolve().parents if (path / "datasets").is_dir())

# Données de base issues de l'AMM PumpSwap (90 jours)
total_sol_communities = 77261
total_sol_fees = 58553979
avg_sol_fees = 757

total_usdc_communities = 682
total_usdc_fees = 1894433
avg_usdc_fees = 2777

total_market_fees = total_sol_fees + total_usdc_fees
total_communities = total_sol_communities + total_usdc_communities

# Variables de Simulation
take_rates = [0.005, 0.01, 0.02, 0.05] # 0.5%, 1%, 2%, 5%
quantum_rates = [0.05, 0.10, 0.20] # 5%, 10%, 20%
penetration_rates = [0.01, 0.05, 0.15, 0.50] # 1%, 5%, 15%, 50%
iterations = 1000

results = []

random.seed(42)

for t in take_rates:
    for q in quantum_rates:
        for p in penetration_rates:
            arr_simulations = []
            for _ in range(iterations):
                target_fees = total_market_fees * p
                
                # Bruit gaussien pour simuler l'impact de la loterie sur une loi de puissance
                # (si une baleine gagne la loterie, l'impact sur le CA est plus fort)
                luck_factor = random.gauss(q, q * 0.5)
                luck_factor = max(0.0, min(1.0, luck_factor))
                
                effective_fees_captured = target_fees * (1 - luck_factor)
                quarterly_revenue = effective_fees_captured * t
                arr = quarterly_revenue * 4
                arr_simulations.append(arr)
            
            arr_simulations.sort()
            mean_arr = statistics.mean(arr_simulations)
            # P5 and P95
            min_arr = arr_simulations[int(len(arr_simulations) * 0.05)]
            max_arr = arr_simulations[int(len(arr_simulations) * 0.95)]
            
            results.append({
                'Take_Rate': f"{t * 100:.1f}%",
                'Quantum_Loto': f"{q * 100:.0f}%",
                'Market_Share': f"{p * 100:.0f}%",
                'ARR_Mean_$': f"${mean_arr:,.0f}",
                'ARR_Worst_Case_$': f"${min_arr:,.0f}",
                'ARR_Best_Case_$': f"${max_arr:,.0f}"
            })

# Save to CSV
csv_file = ROOT / "datasets/curated/odysseum_monte_carlo.csv"
with csv_file.open(mode='w', newline='') as file:
    writer = csv.DictWriter(file, fieldnames=results[0].keys())
    writer.writeheader()
    writer.writerows(results)

# Print markdown table for the top configurations (1% take rate)
print("| Take Rate | Quantum Loto | Market Share | ARR (Worst) | ARR (Mean) | ARR (Best) |")
print("|-----------|--------------|--------------|-------------|------------|------------|")
for r in results:
    if r['Take_Rate'] == '1.0%':
        print(f"| {r['Take_Rate']} | {r['Quantum_Loto']} | {r['Market_Share']} | {r['ARR_Worst_Case_$']} | {r['ARR_Mean_$']} | {r['ARR_Best_Case_$']} |")
