"""Layer 1: PERCEPTION — Sensors read data sources."""
import json
from pathlib import Path
from .schema import Tweet, Verdict, SessionTurn, RawPerception

class HermesXSensors:
    def __init__(self):
        self.tweets_path = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "dataset.jsonl"
        self.verdicts_dir = Path.home() / ".cynic" / "organs" / "hermes" / "x" / "verdicts"
        self.sessions_dir = Path.home() / ".claude" / "projects" / "-home-user-Bureau-CYNIC"

    def perceive(self, observation_count: int = 0) -> RawPerception:
        tweets, verdicts, sessions = [], [], []

        # Tweets
        if self.tweets_path.exists():
            with open(self.tweets_path) as f:
                for line in f:
                    try:
                        obj = json.loads(line)
                        tweets.append(Tweet(
                            id=obj.get("id", ""),
                            text=obj.get("text", ""),
                            author=obj.get("author", ""),
                            created_at=obj.get("created_at", ""),
                            signal_score=obj.get("signal_score"),
                        ))
                    except: pass

        # Verdicts
        if self.verdicts_dir.exists():
            for f in self.verdicts_dir.glob("*.json"):
                try:
                    obj = json.load(open(f))
                    verdicts.append(Verdict(
                        id=obj.get("id", ""),
                        content=obj.get("content", ""),
                        domain=obj.get("domain", ""),
                        q_score=obj.get("q_score", 0),
                        verdict_type=obj.get("verdict_type", ""),
                        timestamp=obj.get("timestamp", ""),
                        created_at=obj.get("created_at", ""),
                    ))
                except: pass

        # Sessions
        if self.sessions_dir.exists():
            for f in self.sessions_dir.glob("*.jsonl"):
                turn_count = 0
                with open(f) as fp:
                    for line in fp:
                        try:
                            obj = json.loads(line)
                            if obj.get('type') in ['user', 'assistant']:
                                turn_count += 1
                                msg = str(obj.get('message', '')).lower()[:100]
                                intent = "refactor" if 'refactor' in msg else "debug" if 'debug' in msg else "feature" if 'feature' in msg else "mixed"
                                sessions.append(SessionTurn(
                                    session_id=f.stem,
                                    turn_count=turn_count,
                                    timestamp=obj.get('timestamp', ''),
                                    intent=intent,
                                    message_length=len(msg),
                                ))
                        except: pass

        return RawPerception(
            timestamp=str(Path.cwd()),
            tweets=tweets,
            verdicts=verdicts,
            sessions=sessions,
            observation_count=observation_count,
        )
