"""Small HTTP/HTML website auditor for local service businesses."""

from __future__ import annotations

import re
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

from ..core.entities import WebsiteAudit


class WebsiteAuditor:
    def audit(self, url: str | None, city: str) -> WebsiteAudit:
        if not url:
            return WebsiteAudit(
                url=None,
                reachable=False,
                title=None,
                has_phone=False,
                has_email=False,
                has_contact_form=False,
                has_quote_language=False,
                has_local_terms=False,
                has_ssl=False,
                mobile_hint=False,
                load_error="missing_url",
            )

        parsed = urlparse(url)
        has_ssl = parsed.scheme == "https"
        try:
            request = Request(url, headers={"User-Agent": "Mozilla/5.0 CYNIC organ-prospect"})
            with urlopen(request, timeout=12) as response:
                content_type = response.headers.get("content-type", "")
                raw = response.read(300_000)
            if "text/html" not in content_type and b"<html" not in raw[:5000].lower():
                return self._unreachable(url, has_ssl, "not_html")
            body = raw.decode("utf-8", errors="replace")
        except (HTTPError, URLError, TimeoutError, OSError) as exc:
            return self._unreachable(url, has_ssl, type(exc).__name__)

        text = re.sub(r"<[^>]+>", " ", body)
        normalized = re.sub(r"\s+", " ", text).lower()
        title = self._title(body)
        return WebsiteAudit(
            url=url,
            reachable=True,
            title=title,
            has_phone=bool(re.search(r"(?:(?:\+33|0)\s*[1-9](?:[\s.-]*\d{2}){4})", text)),
            has_email=bool(re.search(r"[\w.+-]+@[\w.-]+\.[a-zA-Z]{2,}", text)),
            has_contact_form="<form" in body.lower(),
            has_quote_language=any(token in normalized for token in ("devis", "estimation", "rendez-vous", "rdv")),
            has_local_terms=city.lower() in normalized,
            has_ssl=has_ssl,
            mobile_hint="viewport" in body.lower() or "@media" in body.lower(),
            load_error=None,
        )

    def _unreachable(self, url: str, has_ssl: bool, error: str) -> WebsiteAudit:
        return WebsiteAudit(
            url=url,
            reachable=False,
            title=None,
            has_phone=False,
            has_email=False,
            has_contact_form=False,
            has_quote_language=False,
            has_local_terms=False,
            has_ssl=has_ssl,
            mobile_hint=False,
            load_error=error,
        )

    def _title(self, body: str) -> str | None:
        match = re.search(r"<title[^>]*>(.*?)</title>", body, re.IGNORECASE | re.DOTALL)
        if not match:
            return None
        return re.sub(r"\s+", " ", match.group(1)).strip()[:160]

