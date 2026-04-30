"""
Layer 2: TRANSFORMATION — Clean, validate, normalize raw data.

Anti-patterns avoided:
- Silent data loss (we log everything dropped)
- Data corruption (validation at each step)
- Unbounded operations (no CROSS JOIN equivalents)

Inputs: RawPerception
Outputs: CleanedData (typed contract)
"""

from .schema import RawPerception, CleanedData, Tweet, Verdict, SessionTurn


class DataTransformer:
    """Transform raw perception into cleaned, validated data"""

    def __init__(self, min_text_length: int = 3, min_q_score: float = 0.0):
        self.min_text_length = min_text_length
        self.min_q_score = min_q_score

    def clean_tweets(self, tweets: list) -> tuple[list, int]:
        """
        Validate and clean tweets.

        Rules:
        - Must have non-empty text
        - Must have valid created_at timestamp
        - Signal score (if present) must be in [-5, 7]

        Returns:
            (valid_tweets, dropped_count)
        """
        valid = []
        dropped = 0

        for tweet in tweets:
            if not tweet.text or len(tweet.text.strip()) < self.min_text_length:
                dropped += 1
                continue

            if not tweet.created_at:
                dropped += 1
                continue

            # Validate signal score if present
            if tweet.signal_score is not None:
                if not (-5 <= tweet.signal_score <= 7):
                    dropped += 1
                    continue

            valid.append(tweet)

        return valid, dropped

    def clean_verdicts(self, verdicts: list) -> tuple[list, int]:
        """
        Validate and clean verdicts.

        Rules:
        - Must have non-empty content
        - q_score must be in [0.0, 1.0]
        - verdict_type must be one of known types

        Returns:
            (valid_verdicts, dropped_count)
        """
        valid_types = {"HOWL", "BARK", "GROWL", "WAG", "unknown"}
        valid = []
        dropped = 0

        for verdict in verdicts:
            if not verdict.content or not verdict.domain:
                dropped += 1
                continue

            if not (0.0 <= verdict.q_score <= 1.0):
                dropped += 1
                continue

            if verdict.verdict_type not in valid_types:
                dropped += 1
                continue

            valid.append(verdict)

        return valid, dropped

    def clean_sessions(self, sessions: list) -> tuple[list, int]:
        """
        Validate and clean session turns.

        Rules:
        - Must have session_id
        - turn_count must be positive
        - intent must be recognized

        Returns:
            (valid_sessions, dropped_count)
        """
        valid_intents = {"refactor", "debug", "feature", "mixed"}
        valid = []
        dropped = 0

        for session in sessions:
            if not session.session_id:
                dropped += 1
                continue

            if session.turn_count <= 0:
                dropped += 1
                continue

            if session.intent not in valid_intents:
                dropped += 1
                continue

            valid.append(session)

        return valid, dropped

    def transform(self, perception: RawPerception) -> CleanedData:
        """
        Transform raw perception into cleaned data.

        Applies validation rules to each data type.
        Logs dropped records for observability.

        Returns:
            CleanedData with quality metrics
        """
        # Clean each source
        tweets_valid, tweets_dropped = self.clean_tweets(perception.tweets)
        verdicts_valid, verdicts_dropped = self.clean_verdicts(perception.verdicts)
        sessions_valid, sessions_dropped = self.clean_sessions(perception.sessions)

        return CleanedData(
            timestamp=perception.timestamp,
            tweets_valid=tweets_valid,
            tweets_dropped=tweets_dropped,
            verdicts_valid=verdicts_valid,
            verdicts_dropped=verdicts_dropped,
            sessions_valid=sessions_valid,
            sessions_dropped=sessions_dropped,
        )
