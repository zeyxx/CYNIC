"""
Tier 1 EXPERIMENTAL: CCM CHAOS phase — cluster observations by embedding similarity.

Research question: What natural clusters exist in 120K observations?
Success condition: Clusters emerge that are NOT the current domain taxonomy.
Timeline: One session. If no signal → delete.

Approach:
1. Sample observations from SurrealDB (stratified by domain)
2. Embed each observation via Qwen3-Embedding on :8081
3. Reduce with UMAP, cluster with HDBSCAN
4. Compare emergent clusters vs imposed domains
5. Output: cluster report as JSON for human review
"""

import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

import httpx
import numpy as np

SURREAL_URL = "http://localhost:8000/sql"
SURREAL_HEADERS = {
    "Accept": "application/json",
    "surreal-ns": "cynic",
    "surreal-db": "main",
}
SURREAL_AUTH = ("root", "root")

EMBED_URL = os.environ.get("EMBEDDING_URL", "http://localhost:8081/v1/embeddings")
EMBED_API_KEY = os.environ.get("SOVEREIGN_API_KEY", "")
SAMPLE_SIZE = 2000  # enough for clustering, not too expensive
OUTPUT_PATH = Path(__file__).parent / "ccm_chaos_clusters.json"


def query_surreal(sql: str) -> list:
    resp = httpx.post(
        SURREAL_URL,
        headers=SURREAL_HEADERS,
        auth=SURREAL_AUTH,
        content=sql,
        timeout=30.0,
    )
    data = resp.json()
    if isinstance(data, list) and data and data[0].get("status") == "OK":
        return data[0].get("result", [])
    return []


def sample_observations(n: int) -> list[dict]:
    """Stratified sample: proportional to domain size, min 5 per domain."""
    # Get domain distribution
    dist = query_surreal(
        "SELECT domain, count() AS c FROM observation GROUP BY domain ORDER BY c DESC;"
    )
    total = sum(r["c"] for r in dist)

    samples = []
    for r in dist:
        domain = r["domain"]
        count = r["c"]
        # Proportional allocation, min 5, max 200 per domain
        alloc = max(5, min(200, int(n * count / total)))
        rows = query_surreal(
            f"SELECT id, tool, domain, context, agent_id, tags, created_at "
            f"FROM observation WHERE domain = '{domain}' "
            f"ORDER BY RAND() LIMIT {alloc};"
        )
        for row in rows:
            # Build text representation for embedding
            ctx = row.get("context", "") or ""
            text = f"[{row.get('tool','')}] {row.get('domain','')}: {ctx[:300]}"
            row["_text"] = text
            samples.append(row)

    print(f"Sampled {len(samples)} observations from {len(dist)} domains")
    return samples


def embed_batch(texts: list[str], batch_size: int = 32) -> list[list[float]]:
    """Embed texts via Qwen3-Embedding on llama-server."""
    all_embeddings = []
    for i in range(0, len(texts), batch_size):
        batch = texts[i : i + batch_size]
        try:
            headers = {}
            if EMBED_API_KEY:
                headers["Authorization"] = f"Bearer {EMBED_API_KEY}"
            resp = httpx.post(
                EMBED_URL,
                json={"input": batch},
                headers=headers,
                timeout=60.0,
            )
            data = resp.json()
            for item in data.get("data", []):
                all_embeddings.append(item["embedding"])
        except Exception as e:
            print(f"Embed batch {i} failed: {e}", file=sys.stderr)
            # Fill with zeros for failed batch
            all_embeddings.extend([[0.0] * 1024] * len(batch))

        if (i + batch_size) % 200 == 0:
            print(f"  Embedded {min(i + batch_size, len(texts))}/{len(texts)}")

    return all_embeddings


def cluster_embeddings(
    embeddings: np.ndarray,
) -> tuple[np.ndarray, np.ndarray]:
    """UMAP reduce + HDBSCAN cluster."""
    try:
        from umap import UMAP
        from hdbscan import HDBSCAN
    except ImportError:
        print("Installing umap-learn and hdbscan...", file=sys.stderr)
        import subprocess

        subprocess.run(
            [sys.executable, "-m", "pip", "install", "umap-learn", "hdbscan"],
            check=True,
            capture_output=True,
        )
        from umap import UMAP
        from hdbscan import HDBSCAN

    print("UMAP reducing...")
    reducer = UMAP(n_components=15, n_neighbors=30, min_dist=0.0, random_state=42)
    reduced = reducer.fit_transform(embeddings)

    print("HDBSCAN clustering...")
    clusterer = HDBSCAN(min_cluster_size=10, min_samples=5)
    labels = clusterer.fit_predict(reduced)

    # Also get 2D for visualization
    reducer_2d = UMAP(n_components=2, n_neighbors=30, min_dist=0.1, random_state=42)
    coords_2d = reducer_2d.fit_transform(embeddings)

    return labels, coords_2d


def analyze_clusters(
    samples: list[dict], labels: np.ndarray, coords_2d: np.ndarray
) -> dict:
    """Compare emergent clusters vs imposed domains."""
    n_clusters = len(set(labels)) - (1 if -1 in labels else 0)
    noise_count = int(np.sum(labels == -1))

    # Per-cluster analysis
    clusters = {}
    for i, (sample, label) in enumerate(zip(samples, labels)):
        label = int(label)
        if label not in clusters:
            clusters[label] = {
                "domains": Counter(),
                "tools": Counter(),
                "count": 0,
                "examples": [],
            }
        c = clusters[label]
        c["count"] += 1
        c["domains"][sample.get("domain", "?")] += 1
        c["tools"][sample.get("tool", "?")] += 1
        if len(c["examples"]) < 3:
            c["examples"].append(sample["_text"][:150])

    # Format for output
    cluster_report = []
    for label in sorted(clusters.keys()):
        c = clusters[label]
        top_domains = c["domains"].most_common(3)
        top_tools = c["tools"].most_common(3)
        # Domain purity: does one domain dominate?
        total = c["count"]
        top_domain_pct = top_domains[0][1] / total if top_domains else 0

        cluster_report.append(
            {
                "cluster_id": label,
                "size": c["count"],
                "label": "noise" if label == -1 else f"cluster_{label}",
                "domain_purity": round(top_domain_pct, 3),
                "top_domains": [
                    {"domain": d, "count": n, "pct": round(n / total, 3)}
                    for d, n in top_domains
                ],
                "top_tools": [{"tool": t, "count": n} for t, n in top_tools],
                "examples": c["examples"],
            }
        )

    # Domain coverage: how many imposed domains map cleanly to clusters?
    domain_to_clusters: dict[str, set[int]] = {}
    for sample, label in zip(samples, labels):
        d = sample.get("domain", "?")
        domain_to_clusters.setdefault(d, set()).add(int(label))

    domain_fragmentation = []
    for domain in sorted(domain_to_clusters.keys()):
        cluster_set = domain_to_clusters[domain]
        domain_fragmentation.append(
            {
                "domain": domain,
                "n_clusters": len(cluster_set),
                "clusters": sorted(cluster_set),
                "fragmented": len(cluster_set) > 2,
            }
        )

    return {
        "meta": {
            "sample_size": len(samples),
            "n_clusters": n_clusters,
            "noise_count": noise_count,
            "noise_pct": round(noise_count / len(samples), 3),
            "n_imposed_domains": len(domain_to_clusters),
        },
        "clusters": sorted(cluster_report, key=lambda x: -x["size"]),
        "domain_fragmentation": sorted(
            domain_fragmentation, key=lambda x: -x["n_clusters"]
        ),
    }


def main():
    start = time.time()

    print("=== CCM CHAOS Clustering ===")
    print(f"Sample size: {SAMPLE_SIZE}")

    # 1. Sample
    samples = sample_observations(SAMPLE_SIZE)
    if len(samples) < 50:
        print("Too few samples, aborting")
        sys.exit(1)

    # 2. Embed
    print(f"\nEmbedding {len(samples)} observations...")
    texts = [s["_text"] for s in samples]
    embeddings = embed_batch(texts)
    emb_array = np.array(embeddings, dtype=np.float32)
    print(f"Embedding shape: {emb_array.shape}")

    # 3. Cluster
    print("\nClustering...")
    labels, coords_2d = cluster_embeddings(emb_array)

    # 4. Analyze
    print("\nAnalyzing...")
    report = analyze_clusters(samples, labels, coords_2d)

    # 5. Output
    with open(OUTPUT_PATH, "w") as f:
        json.dump(report, f, indent=2)

    elapsed = time.time() - start

    # Print summary
    meta = report["meta"]
    print(f"\n=== Results ({elapsed:.0f}s) ===")
    print(
        f"Clusters: {meta['n_clusters']} emergent (vs {meta['n_imposed_domains']} imposed domains)"
    )
    print(f"Noise: {meta['noise_count']} ({meta['noise_pct']*100:.1f}%)")
    print(f"\nTop clusters:")
    for c in report["clusters"][:10]:
        if c["cluster_id"] == -1:
            continue
        domains = ", ".join(
            f"{d['domain']}({d['pct']*100:.0f}%)" for d in c["top_domains"]
        )
        print(f"  #{c['cluster_id']}: {c['size']} obs — {domains}")
        print(f"    purity={c['domain_purity']:.2f} | ex: {c['examples'][0][:80]}")

    print(f"\nMost fragmented domains (spread across many clusters):")
    for d in report["domain_fragmentation"][:5]:
        if d["fragmented"]:
            print(f"  {d['domain']}: {d['n_clusters']} clusters")

    print(f"\nFull report: {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
