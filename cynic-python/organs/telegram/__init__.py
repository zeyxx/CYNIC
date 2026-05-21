"""
Tier 2 INFRASTRUCTURE: Telegram channel listener — raw message ingestion to SQLite.

K15 Consumer Phase 1: human channel selection gate + kernel silence detection (heartbeat).
K15 Consumer Phase 2: LLM extraction pipeline + Dogs judgment + KAIROS IC measurement.
Systemd: telegram-listener.service
Owns: ~/.cynic/organs/telegram/
"""

__version__ = "0.1.0"
