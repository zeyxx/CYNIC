import json
from types import SimpleNamespace

from organs.twitter.src.entrypoints import gemini_briefing_consumer as briefing


def test_build_parser_accepts_organ_dir(tmp_path):
    args = briefing.build_parser().parse_args(["--organ-dir", str(tmp_path)])

    assert args.organ_dir == str(tmp_path)


def test_load_briefing_reads_latest_analysis(tmp_path):
    analysis_dir = tmp_path / "analysis"
    analysis_dir.mkdir()
    (analysis_dir / "lab_briefing_latest.json").write_text(json.dumps({"ok": True}))

    assert briefing.load_briefing(str(tmp_path)) == {"ok": True}


def test_analyze_domain_priorities_marks_high_under_assigned_domains():
    result = briefing.analyze_domain_priorities(
        {
            "analyses": {
                "distribution": {
                    "D4": {"count": 10},
                    "D5": {"count": 10},
                    "D6": {"count": 1},
                }
            }
        }
    )

    assert result["D4"]["status"] == "under-assigned"
    assert result["D4"]["priority"] == "HIGH"
    assert result["D6"]["deficit"] == 49


def test_create_task_rejects_loopback_kernel(monkeypatch):
    monkeypatch.setenv("CYNIC_REST_ADDR", "localhost:3030")
    monkeypatch.setenv("CYNIC_API_KEY", "test-token")
    monkeypatch.delenv("CYNIC_ALLOW_LOCAL_KERNEL", raising=False)

    assert briefing.create_task("D4", {"analyses": {"distribution": {}}}, "/tmp") is False


def test_create_task_posts_to_normalized_kernel_url(monkeypatch):
    calls = []

    class FakeResponse:
        status = 201

        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

        def read(self):
            return json.dumps({"task_id": "task-1"}).encode("utf-8")

    def fake_urlopen(req, timeout):
        calls.append({"url": req.full_url, "headers": dict(req.header_items()), "timeout": timeout})
        return FakeResponse()

    monkeypatch.setenv("CYNIC_REST_ADDR", "100.64.0.10:3030")
    monkeypatch.setenv("CYNIC_API_KEY", "test-token")
    monkeypatch.setattr(briefing.urllib.request, "urlopen", fake_urlopen)

    assert briefing.create_task("D4", {"analyses": {"distribution": {"D4": {"mean": 0.75}}}}, "/tmp")
    assert calls[0]["url"] == "http://100.64.0.10:3030/agent-tasks"
    assert calls[0]["headers"]["Authorization"] == "Bearer test-token"

