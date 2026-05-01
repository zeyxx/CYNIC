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
        """Use trained LSTM to sample next event (post-training May 5-6)."""
        # Placeholder: would tokenize context, pass through model, sample from output distribution
        # Implementation will be added after training completes
        logger.info("Model inference not yet implemented (training in progress)")
        return self._sample_heuristic(context)

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
