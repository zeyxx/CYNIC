#!/usr/bin/env python3
"""
Extract inference model recommendations from X (Twitter) data.
DATA-CENTRIC approach: real deployment insights from engaged tweets.

This script parses X Organ dataset for inference-related tweets, extracts:
- Model names and sizes
- Hardware platforms (GPU models, VRAM)
- Performance metrics (throughput, latency)
- Tweet engagement (views, likes) as signal strength
- Quantization formats (Q4, Q5, IQ3, etc.)

Output: structured JSON recommendations ranked by engagement + feasibility on target hardware.
"""

import json
import re
import logging
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

logger = logging.getLogger(__name__)


@dataclass
class ModelMention:
    """A single model mention extracted from a tweet."""
    model_name: str  # e.g., "Qwen3.5-9B", "DeepSeek-V3", "Phi-4"
    size_b: Optional[int]  # Model size in billions of params
    quant: Optional[str]  # e.g., "Q4_K_M", "IQ3_XXS", "UD-Q4_K_M"
    hardware: Optional[str]  # e.g., "RTX 4060 Ti", "RTX 3070", "APU"
    vram_gb: Optional[int]  # VRAM in GB if mentioned
    throughput_toks: Optional[float]  # tokens/second if mentioned
    latency_ms: Optional[float]  # milliseconds if mentioned
    tweet_id: str = ""
    author: str = ""
    views: int = 0
    likes: int = 0
    text: str = ""


class InferenceExtractor:
    """Extract model + hardware + performance data from X tweets."""

    # Model name patterns (regex)
    MODEL_PATTERNS = [
        (r"Qwen\s*3\.5[^A-Za-z]*(\d+)?\s*[BGb]", "Qwen3.5"),
        (r"Qwen\s*3\.6[^A-Za-z]*(\d+)?\s*[BGb]", "Qwen3.6"),
        (r"Qwen\s*2\.5[^A-Za-z]*(\d+)?\s*[BGb]", "Qwen2.5"),
        (r"Qwen\s*2\s+(?:VL)?[^A-Za-z]*(\d+)?\s*[BGb]", "Qwen2"),
        (r"DeepSeek[^A-Za-z]*(V\d|R\d)", "DeepSeek"),
        (r"Phi[^A-Za-z]*[14]", "Phi"),
        (r"Gemma\s*2\s*[^A-Za-z]*(\d+)?\s*[BGb]", "Gemma2"),
        (r"Llama\s*3(?:\.[12])?[^A-Za-z]*(\d+)?\s*[BGb]", "Llama3"),
        (r"Mistral[^A-Za-z]*(?:7B|8x)?", "Mistral"),
        (r"Hermes[^A-Za-z]*(\d+)", "Hermes"),
        (r"Grok[^A-Za-z]*(\d+)?", "Grok"),
    ]

    # Hardware patterns
    HARDWARE_PATTERNS = [
        (r"RTX\s*(?:30\d0|40\d0|50\d0|A\d{4}|Pro|6000)\s*(?:Ti)?(?:\s*(\d+)\s*GB)?", "NVIDIA_RTX"),
        (r"H100|A100|A40|L\d{2,}", "NVIDIA_DataCenter"),
        (r"(?:Apple|M\d|MacBook)[^A-Za-z]*(?:(\d+)\s*GB)?", "Apple_Silicon"),
        (r"CPU[^A-Za-z]*(?:only)?", "CPU"),
        (r"APU", "APU"),
        (r"vLLM|ExLlama|Sglang|llama\.cpp", "Framework"),
    ]

    QUANT_PATTERNS = [
        r"Q[3-8]_[KS]_[MS]",
        r"IQ\d_[A-Z]{2,3}",
        r"UD-Q\d_K_M",
        r"[FQ]\d+",
    ]

    THROUGHPUT_PATTERN = r"(\d+(?:\.\d+)?)\s*(?:tokens?/s|tok/s|tps)"
    LATENCY_PATTERN = r"(\d+(?:\.\d+)?)\s*(?:ms|milliseconds)"
    VRAM_PATTERN = r"(\d+)\s*(?:GB|VRAM)"

    def __init__(self, dataset_path: str = "~/.cynic/organs/hermes/x/dataset.jsonl"):
        self.dataset_path = Path(dataset_path).expanduser()
        self.mentions: List[ModelMention] = []
        self.by_model: Dict[str, List[ModelMention]] = defaultdict(list)

    def extract_size(self, text: str, model_name: str) -> Optional[int]:
        """Extract model size in billions from text."""
        # Look for patterns like "7B", "13B", "70B", "9B", etc.
        patterns = [
            rf"{model_name}[^0-9]*(\d+)(?:\.\d+)?\s*[BGb]",
            r"(\d+)(?:\.\d+)?\s*[BGb](?:\s+|\s*,|\s*\))",
        ]
        for pattern in patterns:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                try:
                    return int(match.group(1))
                except (ValueError, IndexError):
                    pass
        return None

    def extract_quant(self, text: str) -> Optional[str]:
        """Extract quantization format."""
        for pattern in self.QUANT_PATTERNS:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                return match.group(0).upper()
        return None

    def extract_metric(self, text: str, pattern: str) -> Optional[float]:
        """Extract numeric metric (throughput, latency, etc.)."""
        match = re.search(pattern, text, re.IGNORECASE)
        if match:
            try:
                return float(match.group(1))
            except (ValueError, IndexError):
                pass
        return None

    def extract_hardware(self, text: str) -> Tuple[Optional[str], Optional[int]]:
        """Extract hardware and VRAM if mentioned."""
        hardware = None
        vram = None

        for pattern, hw_type in self.HARDWARE_PATTERNS:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                hardware = hw_type
                # Try to extract VRAM from the match groups
                if match.lastindex and match.lastindex >= 1:
                    try:
                        vram = int(match.group(1))
                    except (ValueError, IndexError, TypeError):
                        pass
                break

        # Fallback: try generic VRAM pattern
        if not vram:
            vram_match = re.search(self.VRAM_PATTERN, text)
            if vram_match:
                try:
                    vram = int(vram_match.group(1))
                except (ValueError, IndexError):
                    pass

        return hardware, vram

    def extract_from_tweet(self, tweet: Dict) -> List[ModelMention]:
        """Extract all model mentions from a single tweet."""
        text = tweet.get("text", "")
        mentions = []

        # Skip non-inference tweets (heuristic: look for keywords)
        inference_keywords = [
            "inference", "llm", "model", "tok/s", "tokens", "q_score",
            "throughput", "latency", "quantiz", "gguf", "ggml",
            "vllm", "llamacpp", "exllama", "sglang"
        ]
        if not any(kw in text.lower() for kw in inference_keywords):
            return mentions

        # Extract all model mentions
        for pattern, model_base in self.MODEL_PATTERNS:
            for match in re.finditer(pattern, text, re.IGNORECASE):
                model_name = model_base
                size = self.extract_size(text, model_base)
                if size:
                    model_name = f"{model_base}-{size}B"

                mention = ModelMention(
                    model_name=model_name,
                    size_b=size,
                    quant=self.extract_quant(text),
                    hardware=self.extract_hardware(text)[0],
                    vram_gb=self.extract_hardware(text)[1],
                    throughput_toks=self.extract_metric(text, self.THROUGHPUT_PATTERN),
                    latency_ms=self.extract_metric(text, self.LATENCY_PATTERN),
                    tweet_id=tweet.get("id", ""),
                    author=tweet.get("author_screen_name", ""),
                    views=tweet.get("views", 0),
                    likes=tweet.get("likes", 0),
                    text=text[:200],  # Store snippet
                )
                mentions.append(mention)

        return mentions

    def process_dataset(self) -> None:
        """Load and parse X dataset, extracting all model mentions."""
        if not self.dataset_path.exists():
            logger.error(f"Dataset not found: {self.dataset_path}")
            return

        total_tweets = 0
        inference_tweets = 0

        with open(self.dataset_path) as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    tweet = json.loads(line)
                    total_tweets += 1

                    extracted = self.extract_from_tweet(tweet)
                    if extracted:
                        inference_tweets += 1
                        for mention in extracted:
                            self.mentions.append(mention)
                            self.by_model[mention.model_name].append(mention)
                except json.JSONDecodeError:
                    pass

        logger.info(f"Processed {total_tweets} tweets, found {inference_tweets} inference-related, {len(self.mentions)} model mentions")

    def rank_models(self) -> List[Tuple[str, Dict]]:
        """Rank models by engagement (views + likes) and feasibility."""
        results = []

        for model_name, mentions in self.by_model.items():
            # Aggregate stats
            total_views = sum(m.views for m in mentions)
            total_likes = sum(m.likes for m in mentions)
            engagement = total_views + (total_likes * 10)  # Weight likes 10x

            avg_throughput = None
            avg_latency = None
            hardware_set = set()
            vram_set = set()

            throughputs = [m.throughput_toks for m in mentions if m.throughput_toks]
            if throughputs:
                avg_throughput = sum(throughputs) / len(throughputs)

            latencies = [m.latency_ms for m in mentions if m.latency_ms]
            if latencies:
                avg_latency = sum(latencies) / len(latencies)

            hardware_set = {m.hardware for m in mentions if m.hardware}
            vram_set = {m.vram_gb for m in mentions if m.vram_gb}

            results.append((
                model_name,
                {
                    "engagement": engagement,
                    "views": total_views,
                    "likes": total_likes,
                    "mentions": len(mentions),
                    "avg_throughput_toks": avg_throughput,
                    "avg_latency_ms": avg_latency,
                    "hardware_targets": list(hardware_set),
                    "vram_gb": sorted(list(vram_set)),
                    "size_b": mentions[0].size_b if mentions[0].size_b else None,
                }
            ))

        # Sort by engagement descending
        return sorted(results, key=lambda x: x[1]["engagement"], reverse=True)

    def report(self, output_file: str = "observations/x_model_rankings.json") -> None:
        """Generate ranking report."""
        ranked = self.rank_models()

        report = {
            "generated": __import__("datetime").datetime.now().isoformat(),
            "total_mentions": len(self.mentions),
            "models": []
        }

        for i, (model_name, stats) in enumerate(ranked, 1):
            report["models"].append({
                "rank": i,
                "model": model_name,
                **stats
            })

        output_path = Path(output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)

        with open(output_path, "w") as f:
            json.dump(report, f, indent=2)

        logger.info(f"Report saved to {output_file}")

        # Print summary
        print("\n# X Inference Model Rankings (by Twitter Engagement)\n")
        for item in report["models"][:15]:  # Top 15
            print(f"{item['rank']}. **{item['model']}** ({item['size_b']}B)")
            print(f"   - Engagement: {item['engagement']:,.0f} (views: {item['views']:,}, likes: {item['likes']:,})")
            print(f"   - Mentions: {item['mentions']} tweets")
            if item['avg_throughput_toks']:
                print(f"   - Throughput: {item['avg_throughput_toks']:.1f} tok/s (avg)")
            if item['avg_latency_ms']:
                print(f"   - Latency: {item['avg_latency_ms']:.0f}ms (avg)")
            if item['hardware_targets']:
                print(f"   - Hardware: {', '.join(item['hardware_targets'])}")
            if item['vram_gb']:
                print(f"   - VRAM: {item['vram_gb']} GB")
            print()


if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO)

    extractor = InferenceExtractor()
    extractor.process_dataset()
    extractor.report()
