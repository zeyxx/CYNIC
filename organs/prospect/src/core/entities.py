"""Domain entities for local prospect acquisition."""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Any


@dataclass(frozen=True)
class SearchHit:
    title: str
    url: str
    snippet: str = ""


@dataclass(frozen=True)
class WebsiteAudit:
    url: str | None
    reachable: bool
    title: str | None
    has_phone: bool
    has_email: bool
    has_contact_form: bool
    has_quote_language: bool
    has_local_terms: bool
    has_ssl: bool
    mobile_hint: bool
    load_error: str | None = None


@dataclass(frozen=True)
class Prospect:
    business_name: str
    city: str
    category: str
    website: str | None
    source_url: str
    source_title: str
    visible_problem: str
    estimated_offer: str
    priority_score: int
    first_message: str
    audit: WebsiteAudit
    generated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    def to_json(self) -> dict[str, Any]:
        return {
            "schema": "cynic.prospect.v1",
            "business_name": self.business_name,
            "city": self.city,
            "category": self.category,
            "website": self.website,
            "source_url": self.source_url,
            "source_title": self.source_title,
            "visible_problem": self.visible_problem,
            "estimated_offer": self.estimated_offer,
            "priority_score": self.priority_score,
            "first_message": self.first_message,
            "audit": {
                "url": self.audit.url,
                "reachable": self.audit.reachable,
                "title": self.audit.title,
                "has_phone": self.audit.has_phone,
                "has_email": self.audit.has_email,
                "has_contact_form": self.audit.has_contact_form,
                "has_quote_language": self.audit.has_quote_language,
                "has_local_terms": self.audit.has_local_terms,
                "has_ssl": self.audit.has_ssl,
                "mobile_hint": self.audit.mobile_hint,
                "load_error": self.audit.load_error,
            },
            "generated_at": self.generated_at,
        }

