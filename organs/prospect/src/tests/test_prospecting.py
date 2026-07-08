from organs.prospect.src.core.entities import SearchHit, WebsiteAudit
from organs.prospect.src.core.browser_results import urls_from_browser_result
from organs.prospect.src.core.prospecting import generate_prospects


class FakeSearch:
    def search(self, query: str, limit: int):
        return [
            SearchHit(title="Dupont Couverture - Exemple", url="https://dupont.example"),
            SearchHit(title="Dupont Couverture - Exemple", url="https://dupont.example/contact"),
            SearchHit(title="PagesJaunes couvreur Lyon", url="https://www.pagesjaunes.fr/pros/123"),
        ][:limit]


class FakeAuditor:
    def audit(self, url: str | None, city: str):
        return WebsiteAudit(
            url=url,
            reachable=True,
            title="Dupont",
            has_phone=True,
            has_email=False,
            has_contact_form=False,
            has_quote_language=False,
            has_local_terms=city.lower() in "lyon",
            has_ssl=url.startswith("https://") if url else False,
            mobile_hint=False,
            load_error=None,
        )


def test_generate_prospects_dedupes_and_scores():
    prospects = generate_prospects(FakeSearch(), FakeAuditor(), "couvreur", "Lyon", 10)

    assert len(prospects) == 1
    assert prospects[0].to_json()["schema"] == "cynic.prospect.v1"
    assert "audit court" in prospects[0].first_message


def test_urls_from_browser_result_filters_noise_and_dedupes():
    payload = {
        "task_id": "task-1",
        "status": "completed",
        "observed_at": "2026-07-03T00:00:00Z",
        "url": "http://127.0.0.1:8080/search?q=couvreur",
        "mutations_performed": [],
        "evidence": [
            {"kind": "official_website_url", "value": "https://dupont-couverture.example/"},
            {"kind": "source_page_url", "value": "https://www.pagesjaunes.fr/pros/123"},
            {
                "kind": "structured_result",
                "value": '[{"business_name":"Martin","official_website_url":"https://martin-toiture.example/contact"}]',
            },
            {"kind": "text", "value": "also see https://dupont-couverture.example/devis"},
        ],
    }

    assert urls_from_browser_result(payload) == [
        "https://dupont-couverture.example",
        "https://martin-toiture.example/contact",
    ]
