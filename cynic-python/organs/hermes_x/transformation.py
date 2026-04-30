"""Layer 2: TRANSFORMATION — Clean and validate data."""
from .schema import RawPerception, CleanedData

class DataTransformer:
    def transform(self, perception: RawPerception) -> CleanedData:
        # Clean tweets: require non-empty text
        tweets_valid = [t for t in perception.tweets if t.text and t.created_at]
        tweets_dropped = len(perception.tweets) - len(tweets_valid)

        # Clean verdicts: require q_score in [0,1]
        verdicts_valid = [v for v in perception.verdicts if 0 <= v.q_score <= 1 and v.domain]
        verdicts_dropped = len(perception.verdicts) - len(verdicts_valid)

        # Clean sessions: require session_id and turn_count > 0
        sessions_valid = [s for s in perception.sessions if s.session_id and s.turn_count > 0]
        sessions_dropped = len(perception.sessions) - len(sessions_valid)

        return CleanedData(
            timestamp=perception.timestamp,
            tweets_valid=tweets_valid,
            tweets_dropped=tweets_dropped,
            verdicts_valid=verdicts_valid,
            verdicts_dropped=verdicts_dropped,
            sessions_valid=sessions_valid,
            sessions_dropped=sessions_dropped,
        )
