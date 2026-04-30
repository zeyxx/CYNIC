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
                        # Classify source stream from query_name
                        qn = obj.get("query_name", obj.get("source_file", "unknown")) or "unknown"
                        if qn in ("home-feed", "profile-visit", "tweet-thread", "notification"):
                            source_stream = "passive"
                        elif qn.startswith("search:"):
                            source_stream = "autonomous"
                        else:
                            source_stream = "unknown"

                        tweet = Tweet(
                            id=obj.get("tweet_id", obj.get("id", "unknown")),
                            text=obj.get("text", ""),
                            author=obj.get("author_screen_name", obj.get("author", "unknown")),
                            created_at=obj.get("created_at", ""),
                            signal_score=obj.get("signal_score"),
                            narratives=obj.get("narratives", []),
                            source_stream=source_stream,
                            query_name=qn,
                            engaged=bool(obj.get("viewer_favorited") or obj.get("viewer_retweeted") or obj.get("viewer_bookmarked")),
                            viewer_favorited=bool(obj.get("viewer_favorited")),
                            viewer_retweeted=bool(obj.get("viewer_retweeted")),
                            viewer_bookmarked=bool(obj.get("viewer_bookmarked")),
                            author_tier=obj.get("author_tier", "unknown"),
                            author_followers=obj.get("author_followers_count", 0) or 0,
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
    """Perceive Hermes agent decisions from feedback logs"""

    def __init__(self, feedback_log: Path = None):
        self.feedback_log = feedback_log or (
            Path.home() / ".cynic" / "organs" / "hermes" / "x" / "feedback_decision_log.jsonl"
        )

    def perceive(self) -> List[SessionTurn]:
        """
        Read agent feedback log, return typed SessionTurn objects representing agent decisions.

        Format: {timestamp, decision (domain), reason, action}
        Maps decision to domain intent for analysis.
        """
        sessions = []

        if not self.feedback_log.exists():
            return sessions

        try:
            with open(self.feedback_log, 'r') as f:
                decision_count = 0
                for line in f:
                    try:
                        obj = json.loads(line)
                        # Extract decision (domain): D1, D2, D3, social, wallet, security, etc.
                        decision = obj.get("decision", "mixed")
                        timestamp = obj.get("timestamp", "")
                        action = obj.get("action", "accepted")

                        # Map decision to domain intent
                        intent = "mixed"
                        if decision.startswith("D"):
                            intent = "feature"  # Domain judgment
                        elif decision in ["social"]:
                            intent = "feature"
                        elif decision in ["wallet", "security", "infrastructure"]:
                            intent = "debug"

                        # Confidence proxy: infer from action (accepted=0.8, other=0.5)
                        confidence = 0.8 if action == "accepted" else 0.5

                        decision_count += 1
                        turn = SessionTurn(
                            session_id="hermes-feedback",
                            turn_count=decision_count,
                            timestamp=timestamp,
                            intent=intent,
                            message_length=int(confidence * 100),
                        )
                        sessions.append(turn)
                    except (json.JSONDecodeError, KeyError, TypeError, ValueError):
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
