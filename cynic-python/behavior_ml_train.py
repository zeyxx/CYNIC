#!/usr/bin/env python3
"""
Behavior ML Training — LSTM on 538K human browser events.
Tokenizes events → trains 80/10/10 split → exports BehaviorSimulator.
"""

import json
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from pathlib import Path
from typing import List, Dict, Tuple
import logging

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s: %(message)s", datefmt="%H:%M:%S")
logger = logging.getLogger("behavior-ml")

class BehaviorLSTM(nn.Module):
    """LSTM for predicting next event in sequence."""

    def __init__(self, input_dim: int = 48, hidden_dim: int = 128, num_layers: int = 1):
        super().__init__()
        self.lstm = nn.LSTM(input_dim, hidden_dim, num_layers, batch_first=True)
        self.fc_type = nn.Linear(hidden_dim, 5)  # Event type (key, mouse, scroll, click, unknown)
        self.fc_pause = nn.Linear(hidden_dim, 8)  # Pause bucket (log scale)
        self.fc_xy = nn.Linear(hidden_dim, 2)    # X, Y coordinates (normalized)

    def forward(self, x):
        lstm_out, _ = self.lstm(x)
        last_out = lstm_out[:, -1, :]  # Take last output in sequence
        type_logits = self.fc_type(last_out)
        pause_logits = self.fc_pause(last_out)
        xy_pred = self.fc_xy(last_out)
        return type_logits, pause_logits, xy_pred


def load_events(log_file: Path, limit: int = None) -> List[Dict]:
    """Load events from behavior_log.jsonl."""
    events = []
    with open(log_file) as f:
        for i, line in enumerate(f):
            if limit and i >= limit:
                break
            try:
                events.append(json.loads(line))
            except:
                pass
    return events


def compute_pause_ms(prev_event: Dict, curr_event: Dict) -> int:
    """Compute pause between events in milliseconds."""
    try:
        from datetime import datetime
        prev_ts = datetime.fromisoformat(prev_event.get('ts', '').replace('+00:00', ''))
        curr_ts = datetime.fromisoformat(curr_event.get('ts', '').replace('+00:00', ''))
        delta = (curr_ts - prev_ts).total_seconds() * 1000
        return int(np.clip(delta, 1, 10000))
    except:
        return 200


def tokenize_event(event: Dict) -> np.ndarray:
    """Convert single event to 48-dim vector."""
    vec = np.zeros(48, dtype=np.float32)

    # Event type (5 dims, 1-hot)
    event_type_map = {"key": 0, "mouse_move": 1, "scroll": 2, "click": 3, "unknown": 4}
    et = event_type_map.get(event.get('type', 'unknown'), 4)
    vec[et] = 1.0

    # X bucket (10 dims, 1-hot)
    x = event.get('x', 960)
    x_bucket = min(9, max(0, int(x / 192)))
    vec[5 + x_bucket] = 1.0

    # Y bucket (8 dims, 1-hot)
    y = event.get('y', 540)
    y_bucket = min(7, max(0, int(y / 135)))
    vec[15 + y_bucket] = 1.0

    # Pause bucket (8 dims, 1-hot)
    pause_ms = event.get('pause_ms', 200)
    pause_log = np.log10(max(pause_ms, 1))
    pause_bucket = min(7, max(0, int(pause_log * 2)))
    vec[23 + pause_bucket] = 1.0

    # Region type (4 dims, 1-hot)
    window_name = event.get('window_name', '')
    if 'Search' in window_name:
        region = 0
    elif 'Home' in window_name or 'Timeline' in window_name:
        region = 1
    elif 'Detail' in window_name:
        region = 2
    else:
        region = 3
    vec[31 + region] = 1.0

    # Window context (1 dim)
    vec[35] = len(window_name) / 100.0

    # Normalized coordinates (2 dims)
    vec[44] = x / 1920.0
    vec[45] = y / 1080.0

    # Confidence (2 dims)
    vec[46] = 1.0
    vec[47] = 0.5

    return vec


def create_sequences(events: List[Dict], seq_len: int = 20) -> Tuple[np.ndarray, np.ndarray]:
    """Convert events to sequences."""
    for i in range(len(events) - 1):
        events[i]['pause_ms'] = compute_pause_ms(events[i], events[i + 1])

    sequences = []
    labels = []

    for i in range(len(events) - seq_len - 1):
        seq = events[i:i+seq_len]
        next_event = events[i+seq_len]

        features = []
        for event in seq:
            features.append(tokenize_event(event))

        sequences.append(np.array(features, dtype=np.float32))

        # Label: next event type
        event_type = next_event.get('type', 'unknown')
        type_map = {'key': 0, 'mouse_move': 1, 'scroll': 2, 'click': 3, 'unknown': 4}
        labels.append(type_map.get(event_type, 4))

    return np.array(sequences), np.array(labels)


def train_model():
    """Main training loop."""
    log_file = Path.home() / '.cynic/organs/hermes/behavior/behavior_log.jsonl'
    device = 'cuda' if torch.cuda.is_available() else 'cpu'

    logger.info(f"Device: {device}")
    logger.info(f"Loading {log_file}")
    events = load_events(log_file)
    logger.info(f"Loaded {len(events)} events")

    logger.info("Creating sequences...")
    sequences, labels = create_sequences(events)
    logger.info(f"Created {len(sequences)} sequences")

    # Split
    n = len(sequences)
    train_idx = int(0.8 * n)
    val_idx = int(0.9 * n)

    train_seq = torch.tensor(sequences[:train_idx], dtype=torch.float32).to(device)
    train_labels = torch.tensor(labels[:train_idx], dtype=torch.long).to(device)

    val_seq = torch.tensor(sequences[train_idx:val_idx], dtype=torch.float32).to(device)
    val_labels = torch.tensor(labels[train_idx:val_idx], dtype=torch.long).to(device)

    test_seq = torch.tensor(sequences[val_idx:], dtype=torch.float32).to(device)
    test_labels = torch.tensor(labels[val_idx:], dtype=torch.long).to(device)

    logger.info(f"Train: {len(train_seq)}, Val: {len(val_seq)}, Test: {len(test_seq)}")

    model = BehaviorLSTM().to(device)
    optimizer = optim.Adam(model.parameters(), lr=1e-3)
    criterion = nn.CrossEntropyLoss()

    logger.info("Training...")
    batch_size = 256
    epochs = 20
    best_val_loss = float('inf')

    for epoch in range(epochs):
        model.train()
        train_loss = 0
        for i in range(0, len(train_seq), batch_size):
            batch_seq = train_seq[i:i+batch_size]
            batch_labels = train_labels[i:i+batch_size]

            type_logits, _, _ = model(batch_seq)
            loss = criterion(type_logits, batch_labels)

            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            train_loss += loss.item()

        train_loss /= max(1, len(train_seq) // batch_size)

        model.eval()
        with torch.no_grad():
            val_logits, _, _ = model(val_seq)
            val_loss = criterion(val_logits, val_labels).item()
            val_acc = (val_logits.argmax(1) == val_labels).float().mean().item()

        logger.info(f"E{epoch+1:2d} | Train: {train_loss:.4f} | Val: {val_loss:.4f} | Acc: {val_acc:.3f}")

        if val_loss < best_val_loss:
            best_val_loss = val_loss
            torch.save(model.state_dict(), '/tmp/behavior_model_best.pt')

    # Test
    model.load_state_dict(torch.load('/tmp/behavior_model_best.pt'))
    model.eval()
    with torch.no_grad():
        test_logits, _, _ = model(test_seq)
        test_acc = (test_logits.argmax(1) == test_labels).float().mean().item()

    logger.info(f"✓ Test Accuracy: {test_acc:.3f}")

    # Save
    model_path = Path.home() / '.cynic/organs/hermes/x/behavior_model.pt'
    torch.save(model.state_dict(), model_path)
    logger.info(f"✓ Saved to {model_path}")


if __name__ == "__main__":
    train_model()
