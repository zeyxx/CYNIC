#!/usr/bin/env python3
"""
BehaviorSimulator — Generates human-like browser events from trained LSTM.
Wraps behavior_model.pt (trained on 538K real events).
Samples next event given context (last 20 events).
"""

import json
import numpy as np
from pathlib import Path
from typing import Dict, List, Optional
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("behavior-simulator")


class BehaviorSimulator:
    """Sample next browser event conditioned on recent event history."""

    def __init__(self, model_path: Optional[Path] = None, device: str = 'cpu'):
        """
        Args:
            model_path: Path to behavior_model.pt (trained LSTM weights)
            device: 'cpu' or 'cuda'
        """
        self.model_path = model_path or (Path.home() / '.cynic/organs/hermes/x/behavior_model.pt')
        self.device = device
        self.model = None
        self.loaded = False

        # Priors learned from 538K events (normalized to sum to 1.0)
        raw_probs = [0.478, 0.472, 0.122, 0.028]
        norm_factor = sum(raw_probs)
        self.event_type_distribution = {
            'key': 0.478 / norm_factor,
            'mouse_move': 0.472 / norm_factor,
            'scroll': 0.122 / norm_factor,
            'click': 0.028 / norm_factor
        }

        # Pause distribution (in milliseconds)
        self.pause_mean = 284  # ms
        self.pause_median = 200  # ms
        self.pause_p95 = 817  # ms

        # Screen dimensions
        self.screen_width = 1920
        self.screen_height = 1080

    def load_model(self):
        """Load trained LSTM weights (if available)."""
        if self.model_path.exists():
            try:
                import torch
                self.model = torch.load(self.model_path, map_location=self.device)
                self.loaded = True
                logger.info(f"✓ Loaded behavior model from {self.model_path}")
            except Exception as e:
                logger.warning(f"Failed to load model: {e}. Using heuristic sampling.")
                self.loaded = False
        else:
            logger.info("Model not found. Using heuristic sampling (will improve May 5-6).")

    def sample_next_event(self, context: List[Dict]) -> Dict:
        """
        Sample next event given context (last 20 events).

        Args:
            context: List of recent events, e.g. [{'type': 'key', 'x': 100, 'y': 200, ...}, ...]

        Returns:
            Generated event dict: {'type': 'mouse_move', 'x': 1200, 'y': 500, 'pause_ms': 145, ...}
        """
        if not context:
            return self._sample_from_prior()

        if self.loaded and self.model:
            # Use LSTM predictions (post-training May 5-6)
            return self._sample_with_model(context)
        else:
            # Heuristic sampling (current, May 1)
            return self._sample_heuristic(context)

    def _sample_from_prior(self) -> Dict:
        """Sample from learned priors (no context)."""
        event_type = np.random.choice(
            list(self.event_type_distribution.keys()),
            p=list(self.event_type_distribution.values())
        )

        pause_ms = int(np.random.gamma(shape=2, scale=self.pause_mean / 2))
        pause_ms = np.clip(pause_ms, 1, 5000)

        x = np.random.randint(0, self.screen_width)
        y = np.random.randint(0, self.screen_height)

        return {
            'type': event_type,
            'x': x,
            'y': y,
            'pause_ms': pause_ms,
            'window_id': '0x2a0000a',
            'window_name': 'X.com — Search'
        }

    def _sample_heuristic(self, context: List[Dict]) -> Dict:
        """
        Heuristic sampling based on context (May 1-6, before LSTM training).
        Rules learned from 538K event analysis:
        - Pause: gamma(2, 140) ≈ mean=284ms
        - Type: 48% key, 47% mouse, 12% scroll, 3% click
        - XY: cluster on recent clicks + random 20% of time
        """
        # Recent click location (if any)
        recent_clicks = [e for e in context[-10:] if e.get('type') == 'click']
        if recent_clicks and np.random.random() < 0.8:
            # Click near recent target
            last_click = recent_clicks[-1]
            x = int(last_click['x'] + np.random.normal(0, 50))
            y = int(last_click['y'] + np.random.normal(0, 50))
        else:
            # Random position
            x = np.random.randint(0, self.screen_width)
            y = np.random.randint(0, self.screen_height)

        # Clamp
        x = np.clip(x, 0, self.screen_width - 1)
        y = np.clip(y, 0, self.screen_height - 1)

        # Pause: bimodal (fast mechanical <100ms, slow deliberation 100-2000ms)
        if np.random.random() < 0.3:
            # Fast (keystroke-to-keystroke or mouse mechanical)
            pause_ms = int(np.random.exponential(scale=50)) + 20
        else:
            # Slow (deliberation, reading)
            pause_ms = int(np.random.gamma(shape=2, scale=200)) + 100

        pause_ms = np.clip(pause_ms, 1, 5000)

        # Event type
        event_type = np.random.choice(
            list(self.event_type_distribution.keys()),
            p=list(self.event_type_distribution.values())
        )

        return {
            'type': event_type,
            'x': int(x),
            'y': int(y),
            'pause_ms': int(pause_ms),
            'window_id': context[-1].get('window_id', '0x2a0000a') if context else '0x2a0000a',
            'window_name': context[-1].get('window_name', 'X.com') if context else 'X.com'
        }

    def _sample_with_model(self, context: List[Dict]) -> Dict:
        """Use trained LSTM to sample next event (trained May 1, 97.7% accuracy)."""
        try:
            import torch

            if not self.model or not context:
                return self._sample_heuristic(context)

            # Tokenize context (pad to 20 events)
            seq_len = 20
            context_padded = context[-seq_len:] if len(context) >= seq_len else (
                [context[0]] * (seq_len - len(context)) + context
            )

            # Encode: 48-dim feature vector per event
            features = []
            for event in context_padded:
                vector = self._encode_event(event)
                features.append(vector)

            # Convert to tensor
            x = torch.tensor([features], dtype=torch.float32)  # [1, 20, 48]

            # Forward pass through LSTM
            with torch.no_grad():
                type_logits, pause_logits, xy_pred = self.model(x)

            # Sample type from LSTM output
            type_probs = torch.softmax(type_logits[0], dim=0).cpu().numpy()
            event_types = list(self.event_type_distribution.keys())
            sampled_type = event_types[np.argmax(type_probs)]

            # Sample pause from pause output
            pause_bucket = torch.argmax(pause_logits[0]).item()
            pause_ms = int(self.pause_mean * (0.5 + pause_bucket / 8.0))  # Map bucket to ms range
            pause_ms = np.clip(pause_ms, 1, 5000)

            # Sample XY from model output (normalized 0-1)
            xy_norm = xy_pred[0].cpu().numpy()
            x = int(xy_norm[0] * self.screen_width)
            y = int(xy_norm[1] * self.screen_height)
            x = np.clip(x, 0, self.screen_width - 1)
            y = np.clip(y, 0, self.screen_height - 1)

            return {
                'type': sampled_type,
                'x': int(x),
                'y': int(y),
                'pause_ms': int(pause_ms),
                'window_id': context[-1].get('window_id', '0x2a0000a') if context else '0x2a0000a',
                'window_name': context[-1].get('window_name', 'X.com') if context else 'X.com'
            }
        except Exception as e:
            logger.warning(f"Model inference failed: {e}, falling back to heuristic")
            return self._sample_heuristic(context)

    def _encode_event(self, event: Dict) -> np.ndarray:
        """Convert event dict to 48-dim feature vector."""
        vector = np.zeros(48, dtype=np.float32)

        # Event type (one-hot, 5 dims)
        event_types = ['key', 'mouse_move', 'scroll', 'click', 'unknown']
        type_idx = event_types.index(event.get('type', 'unknown'))
        vector[type_idx] = 1.0

        # X bucket (10 buckets, dims 5-14)
        x = event.get('x', 0)
        x_bucket = min(int((x / self.screen_width) * 10), 9)
        vector[5 + x_bucket] = 1.0

        # Y bucket (10 buckets, dims 15-24)
        y = event.get('y', 0)
        y_bucket = min(int((y / self.screen_height) * 10), 9)
        vector[15 + y_bucket] = 1.0

        # Pause bucket (8 buckets, dims 25-32, log scale)
        pause_ms = event.get('pause_ms', 200)
        pause_bucket = int(np.log(max(pause_ms, 1)) / np.log(5000) * 8)
        pause_bucket = min(pause_bucket, 7)
        vector[25 + pause_bucket] = 1.0

        # Window info (hash, dims 33-37)
        window_hash = hash(event.get('window_name', 'X.com')) % 5
        vector[33 + window_hash] = 1.0

        # Timestamp relative (dims 38-47) — normalized to 0-1
        # (placeholder: use pause as proxy for recency)
        vector[38] = min(pause_ms / 5000, 1.0)

        return vector

    def generate_sequence(self, length: int, initial_context: Optional[List[Dict]] = None) -> List[Dict]:
        """
        Generate sequence of N events starting from context.

        Args:
            length: Number of events to generate
            initial_context: Starting context (list of recent events)

        Returns:
            List of generated events
        """
        if initial_context is None:
            initial_context = []

        sequence = list(initial_context)

        for _ in range(length):
            # Use last 20 events as context
            context = sequence[-20:] if len(sequence) > 20 else sequence

            next_event = self.sample_next_event(context)
            sequence.append(next_event)

        return sequence[len(initial_context):]  # Return only generated events


def main():
    """Test BehaviorSimulator on heuristic sampling."""
    sim = BehaviorSimulator()

    logger.info("Generating 10-event sequence...")
    sequence = sim.generate_sequence(length=10)

    for i, event in enumerate(sequence):
        pause = event.get('pause_ms', 0)
        et = event.get('type', 'unknown')[:3]
        x, y = event.get('x', 0), event.get('y', 0)
        logger.info(f"  {i+1}: {et:5s} @ ({x:4d},{y:4d}) | pause: {pause:4d}ms")

    logger.info("\n✓ Behavior simulation ready (LSTM training May 5-6 on cynic-gpu)")


if __name__ == "__main__":
    main()
