"""
Layer 1: PERCEPTION — Sensors that perceive the environment.

Reads from:
1. X organ tweets (dataset.jsonl)
2. Kernel verdicts (verdicts/ directory)
3. Claude Code sessions (sessions JSONL files)

Outputs: RawPerception (typed contract)
"""

import json
from pathlib import Path
from datetime import datetime
from typing import List
from .schema import Tweet, Verdict, SessionTurn, BehaviorEvent, RawPerception


class TweetSensor:
    """Perceive tweets from X organ dataset"""

    def __init__(self, dataset_path: Path = None):
        self.dataset_path = dataset_path or (
            Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"
        )

    def perceive(self) -> List[Tweet]:
        """Read tweet dataset, return typed Tweet objects"""
        tweets = []

        if not self.dataset_path.exists():
            return tweets

        try:
            with open(self.dataset_path, 'r') as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        tweet = Tweet(
                            id=obj.get("id", "unknown"),
                            text=obj.get("text", ""),
                            author=obj.get("author", "unknown"),
                            created_at=obj.get("created_at", ""),
                            signal_score=obj.get("signal_score"),
                            narrative=obj.get("narrative"),
                        )
                        tweets.append(tweet)
                    except (json.JSONDecodeError, KeyError):
                        pass
        except Exception:
            pass

        return tweets


class VerdictSensor:
    """Perceive verdicts from kernel verdicts directory"""

    def __init__(self, verdicts_dir: Path = None):
        self.verdicts_dir = verdicts_dir or (
            Path.home() / ".cynic" / "organs" / "hermes" / "x" / "verdicts"
        )

    def perceive(self) -> List[Verdict]:
        """Read verdict files, return typed Verdict objects"""
        verdicts = []

        if not self.verdicts_dir.exists():
            return verdicts

        try:
            for verdict_file in self.verdicts_dir.glob("*.json"):
                try:
                    with open(verdict_file, 'r') as f:
                        obj = json.load(f)
                        # Kernel emits nested structure: obj["verdict"] contains the actual verdict
                        v = obj.get("verdict", {})

                        # Extract q_score from nested structure (may be object or float)
                        q_score_raw = v.get("q_score", 0.0)
                        q_score = q_score_raw.get("total", 0.0) if isinstance(q_score_raw, dict) else q_score_raw

                        # Content is the judgment text: v["verdict"]
                        content = v.get("verdict", "")
                        domain = v.get("domain", "general")
                        verdict_type = v.get("verdict", "unknown").upper()  # Capitalize: "Bark" -> "BARK"

                        verdict = Verdict(
                            id=v.get("verdict_id", verdict_file.stem),
                            content=content,
                            domain=domain,
                            q_score=q_score,
                            verdict_type=verdict_type,
                            timestamp=v.get("timestamp", ""),
                            created_at=obj.get("captured_at", ""),
                        )
                        verdicts.append(verdict)
                except (json.JSONDecodeError, KeyError, TypeError, AttributeError):
                    pass
        except Exception:
            pass

        return verdicts


class HermesAgentSensor:
    """Perceive Hermes agent logs: domains explored, routing, confidence, skill evolution"""

    def __init__(self, logs_dir: Path = None):
        self.logs_dir = logs_dir or (
            Path.home() / ".cynic" / "organs" / "hermes" / "logs"
        )

    def perceive(self) -> List[SessionTurn]:
        """Read Hermes agent logs, return typed SessionTurn objects representing agent decisions"""
        sessions = []

        if not self.logs_dir.exists():
            return sessions

        try:
            # Look for Hermes agent decision logs (future: from /observe endpoint)
            # For now, gracefully return empty if logs don't exist
            for log_file in self.logs_dir.glob("*.jsonl"):
                try:
                    with open(log_file, 'r') as f:
                        decision_count = 0
                        for line in f:
                            try:
                                obj = json.loads(line)
                                # Expected structure: domain, action, confidence
                                domain = obj.get("domain", "mixed")
                                action = obj.get("action", "mixed")
                                confidence = obj.get("confidence", 0.0)

                                # Map domain to analysis intent
                                intent = "mixed"
                                if domain == "social":
                                    intent = "feature"
                                elif domain in ["wallet", "security"]:
                                    intent = "debug"

                                decision_count += 1
                                turn = SessionTurn(
                                    session_id=log_file.stem,
                                    turn_count=decision_count,
                                    timestamp=obj.get("timestamp", ""),
                                    intent=intent,
                                    message_length=int(confidence * 100),  # Normalize confidence to [0,100]
                                )
                                sessions.append(turn)
                            except (json.JSONDecodeError, KeyError, TypeError):
                                pass
                except Exception:
                    pass
        except Exception:
            pass

        return sessions


class BehaviorSensor:
    """Perceive user behavior events from behavior_logger and aggregate by domain"""

    def __init__(self, behavior_path: Path = None):
        self.behavior_path = behavior_path or (
            Path.home() / ".cynic" / "organs" / "hermes" / "behavior" / "behavior_log.jsonl"
        )

    def perceive(self) -> List[BehaviorEvent]:
        """
        Read behavior log and aggregate events by domain/window.

        Raw log has: type (mouse_move, click, etc), window_name (context), ts
        Maps to: event_type, target (from window_name), duration_ms (event frequency as proxy for dwell)
        """
        events = []
        window_event_count = {}

        if not self.behavior_path.exists():
            return events

        try:
            with open(self.behavior_path, 'r') as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        event_type = obj.get("type", "unknown")
                        window_name = obj.get("window_name", "unknown")
                        timestamp = obj.get("ts", "")

                        # Extract domain/target from window_name (last word, lowercased)
                        target = window_name.split()[-1].lower() if window_name and window_name != "unknown" else "unknown"

                        # Count events per window
                        window_key = window_name
                        window_event_count[window_key] = window_event_count.get(window_key, 0) + 1

                        # Emit on significant events
                        if event_type in ["click", "scroll", "key"]:
                            event = BehaviorEvent(
                                event_type=event_type,
                                target=target,
                                duration_ms=1,
                                timestamp=timestamp,
                            )
                            events.append(event)
                        elif event_type == "mouse_move" and window_event_count[window_key] % 100 == 0:
                            # Sample mouse moves
                            event = BehaviorEvent(
                                event_type="dwell",
                                target=target,
                                duration_ms=100,
                                timestamp=timestamp,
                            )
                            events.append(event)

                    except (json.JSONDecodeError, KeyError, ValueError, TypeError, AttributeError):
                        pass
        except Exception:
            pass

        return events


class HermesXSensors:
    """Composite sensor: perceives X tweets, kernel verdicts, Hermes agent logs, user behavior"""

    def __init__(self):
        self.tweet_sensor = TweetSensor()
        self.verdict_sensor = VerdictSensor()
        self.hermes_sensor = HermesAgentSensor()
        self.behavior_sensor = BehaviorSensor()

    def perceive(self, observation_count: int = 0) -> RawPerception:
        """
        Perceive all data sources: X tweets, kernel verdicts, Hermes agent decisions, user behavior.

        Args:
            observation_count: number of kernel observations (for reference)

        Returns:
            RawPerception with all typed data
        """
        return RawPerception(
            timestamp=datetime.now().isoformat(),
            tweets=self.tweet_sensor.perceive(),
            verdicts=self.verdict_sensor.perceive(),
            sessions=self.hermes_sensor.perceive(),  # Hermes agent logs
            behavior=self.behavior_sensor.perceive(),  # User behavior events
            observation_count=observation_count,
        )
