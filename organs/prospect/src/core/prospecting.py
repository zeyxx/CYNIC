"""Prospect search, audit, scoring, and message composition."""

from __future__ import annotations

import re
from urllib.parse import urlparse

from .entities import Prospect, SearchHit, WebsiteAudit
from .ports import WebsiteAuditPort, WebSearchPort


DIRECTORY_HOST_HINTS = (
    "pagesjaunes.fr",
    "societe.com",
    "verif.com",
    "manageo.fr",
    "annuaire",
    "yelp.",
    "118000.fr",
)

NON_PROSPECT_HOST_HINTS = (
    "letudiant.fr",
    "wikipedia.org",
    "oniseptv.onisep.fr",
    "onisep.fr",
    "hellowork.com",
    "meteojob.com",
    "indeed.com",
    "fr.linkedin.com/jobs",
    "larousse.fr",
    "ootravaux.fr",
)


def build_queries(category: str, city: str, limit: int) -> list[str]:
    base = [
        f"{category} {city}",
        f"{category} devis {city}",
        f"{category} urgence {city}",
        f"artisan {category} {city}",
    ]
    return base[: max(1, min(len(base), limit))]


def is_directory_url(url: str) -> bool:
    normalized = url.lower()
    host = urlparse(url).netloc.lower()
    return any(hint in host or hint in normalized for hint in DIRECTORY_HOST_HINTS)


def is_non_prospect_url(url: str) -> bool:
    normalized = url.lower()
    return is_directory_url(url) or any(hint in normalized for hint in NON_PROSPECT_HOST_HINTS)


def business_name_from_hit(hit: SearchHit, category: str, city: str) -> str:
    title = re.sub(r"\s+", " ", hit.title).strip(" .-|:")
    generic = re.search(
        r"(top entreprises|annuaire|artisans locaux|pagesjaunes|comparez|liste|meilleurs)",
        title,
        flags=re.IGNORECASE,
    )
    if generic:
        host = urlparse(hit.url).netloc.lower().removeprefix("www.")
        return host or title[:80] or "prospect"
    title = re.sub(r"\s+[|]\s+.*$", "", title).strip()
    title = re.sub(r"\s+-\s+(Recherche|Bing|Google).*$", "", title, flags=re.IGNORECASE).strip()
    return title[:120] or "prospect"


def visible_problem(audit: WebsiteAudit, source_url: str) -> str:
    if not audit.url:
        return "pas de site identifiable depuis le resultat de recherche"
    if not audit.reachable:
        return "site inaccessible ou erreur au chargement"
    missing = []
    if not audit.has_ssl:
        missing.append("pas de HTTPS")
    if not audit.has_contact_form:
        missing.append("pas de formulaire visible")
    if not audit.has_quote_language:
        missing.append("pas d'appel clair a la demande de devis")
    if not audit.mobile_hint:
        missing.append("optimisation mobile non evidente")
    if is_directory_url(source_url):
        missing.append("dependance a un annuaire plutot qu'a un site propre")
    return ", ".join(missing) if missing else "presence web existante mais ameliorable localement"


def estimated_offer(category: str, problem: str) -> str:
    if "pas de site" in problem or "annuaire" in problem:
        return f"site local {category} + fiche Google + formulaire devis + suivi des demandes"
    if "formulaire" in problem or "devis" in problem:
        return "optimisation conversion + formulaire devis + automatisation tri/reponse"
    return "audit local SEO + pages services/villes + suivi prospects"


def priority_score(audit: WebsiteAudit, source_url: str) -> int:
    score = 50
    if not audit.url:
        score += 30
    if is_directory_url(source_url):
        score += 15
    if audit.url and not audit.reachable:
        score += 20
    if audit.reachable and not audit.has_contact_form:
        score += 12
    if audit.reachable and not audit.has_quote_language:
        score += 10
    if audit.reachable and not audit.mobile_hint:
        score += 8
    if audit.url and not audit.has_ssl:
        score += 8
    if audit.has_phone or audit.has_email:
        score += 4
    return max(0, min(100, score))


def first_message(name: str, category: str, city: str, problem: str, offer: str) -> str:
    return (
        f"Bonjour, j'ai regarde votre presence en ligne pour {category} a {city}. "
        f"J'ai repere un point concret: {problem}. "
        f"Je peux vous montrer gratuitement 3 corrections rapides, puis mettre en place "
        f"{offer} si c'est utile. Vous voulez que je vous envoie l'audit court ?"
    )


def dedupe_hits(hits: list[SearchHit]) -> list[SearchHit]:
    seen: set[str] = set()
    out: list[SearchHit] = []
    for hit in hits:
        key = urlparse(hit.url).netloc.lower().removeprefix("www.")
        if not key or key in seen:
            continue
        seen.add(key)
        out.append(hit)
    return out


def generate_prospects(
    search: WebSearchPort,
    auditor: WebsiteAuditPort,
    category: str,
    city: str,
    max_results: int,
) -> list[Prospect]:
    hits: list[SearchHit] = []
    per_query = max(5, max_results)
    for query in build_queries(category, city, limit=4):
        hits.extend(search.search(query, limit=per_query))

    prospects: list[Prospect] = []
    for hit in dedupe_hits(hits):
        if is_non_prospect_url(hit.url):
            continue
        audit = auditor.audit(hit.url, city=city)
        problem = visible_problem(audit, hit.url)
        offer = estimated_offer(category, problem)
        name = business_name_from_hit(hit, category, city)
        prospects.append(
            Prospect(
                business_name=name,
                city=city,
                category=category,
                website=audit.url,
                source_url=hit.url,
                source_title=hit.title,
                visible_problem=problem,
                estimated_offer=offer,
                priority_score=priority_score(audit, hit.url),
                first_message=first_message(name, category, city, problem, offer),
                audit=audit,
            )
        )

    return sorted(prospects, key=lambda p: p.priority_score, reverse=True)[:max_results]


def generate_prospects_from_urls(
    auditor: WebsiteAuditPort,
    urls: list[str],
    category: str,
    city: str,
) -> list[Prospect]:
    prospects: list[Prospect] = []
    for url in urls:
        hit = SearchHit(title=urlparse(url).netloc or url, url=url, snippet="")
        audit = auditor.audit(url, city=city)
        problem = visible_problem(audit, url)
        offer = estimated_offer(category, problem)
        name = business_name_from_hit(hit, category, city)
        prospects.append(
            Prospect(
                business_name=name,
                city=city,
                category=category,
                website=audit.url,
                source_url=url,
                source_title=hit.title,
                visible_problem=problem,
                estimated_offer=offer,
                priority_score=priority_score(audit, url),
                first_message=first_message(name, category, city, problem, offer),
                audit=audit,
            )
        )
    return sorted(prospects, key=lambda p: p.priority_score, reverse=True)
