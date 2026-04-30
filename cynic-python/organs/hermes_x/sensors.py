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
from .schema import Tweet, Verdict, SessionTurn, RawPerception


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
                        verdict = Verdict(
                            id=obj.get("id", verdict_file.stem),
                            content=obj.get("content", ""),
                            domain=obj.get("domain", "general"),
                            q_score=obj.get("q_score", 0.0),
                            verdict_type=obj.get("verdict_type", "unknown"),
                            timestamp=obj.get("timestamp", ""),
                            created_at=obj.get("created_at", ""),
                        )
                        verdicts.append(verdict)
                except (json.JSONDecodeError, KeyError):
                    pass
        except Exception:
            pass

        return verdicts


class SessionSensor:
    """Perceive Claude Code sessions"""

    def __init__(self, sessions_dir: Path = None):
        self.sessions_dir = sessions_dir or (
            Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC"
        )

    def perceive(self) -> List[SessionTurn]:
        """Read session files, return typed SessionTurn objects"""
        sessions = []

        if not self.sessions_dir.exists():
            return sessions

        try:
            for session_file in self.sessions_dir.glob("*.jsonl"):
                try:
                    with open(session_file, 'r') as f:
                        turn_count = 0
                        for line in f:
                            try:
                                obj = json.loads(line)
                                if obj.get('type') in ['user', 'assistant']:
                                    turn_count += 1
                                    msg = str(obj.get('message', ''))[:100].lower()

                                    # Infer intent
                                    intent = "mixed"
                                    if 'refactor' in msg:
                                        intent = "refactor"
                                    elif 'debug' in msg or 'error' in msg:
                                        intent = "debug"
                                    elif 'feature' in msg or 'add' in msg:
                                        intent = "feature"

                                    turn = SessionTurn(
                                        session_id=session_file.stem,
                                        turn_count=turn_count,
                                        timestamp=obj.get('timestamp', ''),
                                        intent=intent,
                                        message_length=len(msg),
                                    )
                                    sessions.append(turn)
                            except (json.JSONDecodeError, KeyError, TypeError):
                                pass
                except Exception:
                    pass
        except Exception:
            pass

        return sessions


class HermesXSensors:
    """Composite sensor: perceives all data sources"""

    def __init__(self):
        self.tweet_sensor = TweetSensor()
        self.verdict_sensor = VerdictSensor()
        self.session_sensor = SessionSensor()

    def perceive(self, observation_count: int = 0) -> RawPerception:
        """
        Perceive all data sources.

        Args:
            observation_count: number of kernel observations (for reference)

        Returns:
            RawPerception with all typed data
        """
        return RawPerception(
            timestamp=datetime.now().isoformat(),
            tweets=self.tweet_sensor.perceive(),
            verdicts=self.verdict_sensor.perceive(),
            sessions=self.session_sensor.perceive(),
            observation_count=observation_count,
        )
