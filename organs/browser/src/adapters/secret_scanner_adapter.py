import re
from ..core.ports import SecretScannerPort

class RegexSecretScannerAdapter(SecretScannerPort):
    def __init__(self):
        self.patterns = [
            (re.compile(r'\b[1-9A-HJ-NP-Za-km-z]{44,50}\b'), "[REDACTED_BASE58_KEY]"),
            (re.compile(r'\[\s*(?:\d+\s*,\s*){31,63}\d+\s*\]'), "[REDACTED_KEYPAIR_ARRAY]"),
            (re.compile(r'\b[a-fA-F0-9]{64}\b'), "[REDACTED_HEX_KEY]"),
            (re.compile(r'sk-[a-zA-Z0-9]{32,48}'), "[REDACTED_OPENAI_KEY]"),
            (re.compile(r'AIzaSy[a-zA-Z0-9\-_]{33}'), "[REDACTED_GOOGLE_API_KEY]"),
            (re.compile(r'ghp_[a-zA-Z0-9]{36,40}'), "[REDACTED_GITHUB_TOKEN]"),
            (re.compile(r'(?i)cookie:\s*[^\s;=]+=[^\s;]+(?:;\s*[^\s;=]+=[^\s;]+)*'), "Cookie: [REDACTED_COOKIES]")
        ]

    def sanitize(self, text: str) -> str:
        if not text:
            return ""
        sanitized = text
        for pattern, replacement in self.patterns:
            sanitized = pattern.sub(replacement, sanitized)
        return sanitized
