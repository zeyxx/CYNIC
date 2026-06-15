#!/usr/bin/env python3
"""Local UI for the organ-discord opportunity queue."""
from __future__ import annotations

import html
import json
import os
import sqlite3
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

DB_PATH = Path(os.environ.get("DISCORD_DB_PATH", "~/.cynic/organs/discord/messages.db")).expanduser()
HOST = os.environ.get("DISCORD_UI_HOST", "127.0.0.1")
PORT = int(os.environ.get("DISCORD_UI_PORT", "8891"))

STATUSES = ("new", "reviewing", "applied", "won", "lost", "ignored")
CAPABILITIES = ("backend", "frontend", "mobile", "content", "growth", "design", "general")


def connect() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn


def ensure_schema(conn: sqlite3.Connection) -> None:
    from .schema import SCHEMA_SQL, migrate_db
    from .skills import bootstrap_skill_assessments

    conn.executescript(SCHEMA_SQL)
    migrate_db(conn)
    bootstrap_skill_assessments(conn)
    conn.commit()


def rows(status: str | None = None, capability: str | None = None) -> list[sqlite3.Row]:
    conn = connect()
    ensure_schema(conn)
    params: list[str] = []
    where = ""
    if status and status in STATUSES:
        where = "WHERE status = ?"
        params.append(status)
    if capability and capability in CAPABILITIES:
        where += " AND " if where else "WHERE "
        where += "capability = ?"
        params.append(capability)
    data = conn.execute(
        f"""SELECT title, category, capability, reward, reward_usd, url, fit_score, status,
                  platform_status, platform_last_seen, platform_title,
                  platform_excerpt, first_seen, last_seen, notes
             FROM opportunities
             {where}
             ORDER BY
               CASE status
                 WHEN 'reviewing' THEN 0
                 WHEN 'new' THEN 1
                 WHEN 'applied' THEN 2
                 WHEN 'won' THEN 3
                 WHEN 'lost' THEN 4
                 ELSE 5
               END,
               fit_score DESC,
               COALESCE(reward_usd, 0) DESC,
               last_seen DESC""",
        params,
    ).fetchall()
    conn.close()
    return data


def counts() -> dict[str, int]:
    conn = connect()
    ensure_schema(conn)
    result = {status: 0 for status in STATUSES}
    for status, count in conn.execute(
        "SELECT status, count(*) FROM opportunities GROUP BY status"
    ).fetchall():
        result[status] = count
    conn.close()
    return result


def capability_counts() -> dict[str, int]:
    conn = connect()
    ensure_schema(conn)
    result = {capability: 0 for capability in CAPABILITIES}
    for capability, count in conn.execute(
        "SELECT capability, count(*) FROM opportunities GROUP BY capability"
    ).fetchall():
        result[capability] = count
    conn.close()
    return result


def update(url: str, status: str, notes: str | None = None) -> None:
    if status not in STATUSES:
        return
    conn = connect()
    ensure_schema(conn)
    if notes is None:
        conn.execute("UPDATE opportunities SET status = ? WHERE url = ?", (status, url))
    else:
        conn.execute(
            "UPDATE opportunities SET status = ?, notes = ? WHERE url = ?",
            (status, notes, url),
        )
    conn.commit()
    conn.close()


class Handler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/api/opportunities":
            self.respond_json({"opportunities": [dict(row) for row in rows()]})
            return
        if parsed.path == "/api/skills":
            self.respond_json({"skills": [serialize_skill_row(row) for row in skill_rows()]})
            return
        if parsed.path == "/skills":
            self.respond(render_skills())
            return
        if parsed.path != "/":
            self.send_error(404)
            return
        query = parse_qs(parsed.query)
        status = query.get("status", [None])[0]
        capability = query.get("capability", [None])[0]
        self.respond(render(status, capability))

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path != "/update":
            self.send_error(404)
            return
        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length).decode("utf-8")
        form = parse_qs(body)
        update(
            form.get("url", [""])[0],
            form.get("status", ["new"])[0],
            form.get("notes", [None])[0],
        )
        self.send_response(303)
        self.send_header("Location", "/")
        self.end_headers()

    def respond(self, body: str) -> None:
        payload = body.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def respond_json(self, body: object) -> None:
        payload = json.dumps(body, separators=(",", ":")).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "http://127.0.0.1:5000")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, fmt: str, *args: object) -> None:
        return


def render(active_status: str | None, active_capability: str | None) -> str:
    all_rows = rows(active_status, active_capability)
    count_map = counts()
    capability_map = capability_counts()
    nav = " ".join(
        f'<a class="pill" href="/?status={status}">{status} {count_map.get(status, 0)}</a>'
        for status in STATUSES
    )
    capability_nav = " ".join(
        f'<a class="pill" href="/?capability={capability}">{capability} {capability_map.get(capability, 0)}</a>'
        for capability in CAPABILITIES
    )
    cards = "\n".join(render_card(row) for row in all_rows)
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CYNIC Opportunities</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #101312;
      --panel: #171b19;
      --line: #303832;
      --text: #e8eee9;
      --muted: #9ba79f;
      --accent: #9bd67a;
      --warn: #e5c76b;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--bg);
      color: var(--text);
    }}
    header {{
      position: sticky;
      top: 0;
      z-index: 2;
      background: rgba(16, 19, 18, 0.96);
      border-bottom: 1px solid var(--line);
      padding: 16px 24px;
    }}
    h1 {{ margin: 0 0 12px; font-size: 20px; font-weight: 650; }}
    .nav {{ display: flex; flex-wrap: wrap; gap: 8px; }}
    .pill {{
      color: var(--text);
      border: 1px solid var(--line);
      text-decoration: none;
      padding: 6px 10px;
      border-radius: 6px;
      font-size: 13px;
    }}
    main {{
      max-width: 1180px;
      margin: 0 auto;
      padding: 18px 18px 40px;
      display: grid;
      gap: 12px;
    }}
    article {{
      border: 1px solid var(--line);
      background: var(--panel);
      border-radius: 8px;
      padding: 14px;
      display: grid;
      grid-template-columns: minmax(0, 1fr) 230px;
      gap: 16px;
    }}
    h2 {{ margin: 0 0 8px; font-size: 16px; line-height: 1.35; }}
    .meta {{ display: flex; flex-wrap: wrap; gap: 8px; color: var(--muted); font-size: 13px; }}
    .score {{ color: var(--accent); font-weight: 700; }}
    .reward {{ color: var(--warn); }}
    .actions {{ display: grid; gap: 8px; }}
    select, textarea, button, .open {{
      width: 100%;
      border-radius: 6px;
      border: 1px solid var(--line);
      background: #0f1211;
      color: var(--text);
      padding: 8px;
      font: inherit;
      font-size: 13px;
    }}
    textarea {{ min-height: 70px; resize: vertical; }}
    button {{ cursor: pointer; background: #20351f; }}
    .open {{ display: block; text-align: center; text-decoration: none; }}
    @media (max-width: 820px) {{
      article {{ grid-template-columns: 1fr; }}
      header {{ padding: 14px; }}
      main {{ padding: 12px; }}
    }}
  </style>
</head>
<body>
  <header>
    <h1>CYNIC Opportunity Queue</h1>
    <nav class="nav"><a class="pill" href="/">opportunities</a><a class="pill" href="/skills">skills</a><a class="pill" href="/">all</a>{nav}</nav>
    <nav class="nav" style="margin-top:8px">{capability_nav}</nav>
  </header>
  <main>{cards or '<p>No opportunities yet.</p>'}</main>
</body>
</html>"""


def skill_rows() -> list[sqlite3.Row]:
    conn = connect()
    ensure_schema(conn)
    data = conn.execute(
        """SELECT c.category, c.skill, c.source, a.level, a.confidence, a.status,
                  a.evidence_json, a.judge_verdict_json, a.updated_at
             FROM skill_catalog c
             LEFT JOIN skill_assessments a
               ON a.category = c.category AND a.skill = c.skill
             ORDER BY
               c.category,
               CASE c.skill WHEN 'All' THEN 0 ELSE 1 END,
               COALESCE(a.level, 0) DESC,
               c.skill"""
    ).fetchall()
    conn.close()
    return data


def render_skills() -> str:
    grouped: dict[str, list[sqlite3.Row]] = {}
    for row in skill_rows():
        grouped.setdefault(row["category"], []).append(row)

    sections = "\n".join(
        f"""<section class="skill-section">
  <h2>{html.escape(category)}</h2>
  <div class="skill-grid">
    {''.join(render_skill_card(row) for row in rows)}
  </div>
</section>"""
        for category, rows in grouped.items()
    )
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CYNIC Skills</title>
  <style>
    :root {{
      color-scheme: dark;
      --bg: #101312;
      --panel: #171b19;
      --line: #303832;
      --text: #e8eee9;
      --muted: #9ba79f;
      --accent: #9bd67a;
      --warn: #e5c76b;
      --low: #c76f63;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--bg);
      color: var(--text);
    }}
    header {{
      position: sticky;
      top: 0;
      z-index: 2;
      background: rgba(16, 19, 18, 0.96);
      border-bottom: 1px solid var(--line);
      padding: 16px 24px;
    }}
    h1 {{ margin: 0 0 12px; font-size: 20px; font-weight: 650; }}
    .nav {{ display: flex; flex-wrap: wrap; gap: 8px; }}
    .pill {{
      color: var(--text);
      border: 1px solid var(--line);
      text-decoration: none;
      padding: 6px 10px;
      border-radius: 6px;
      font-size: 13px;
    }}
    main {{
      max-width: 1180px;
      margin: 0 auto;
      padding: 18px 18px 40px;
      display: grid;
      gap: 22px;
    }}
    .skill-section h2 {{ margin: 0 0 10px; font-size: 18px; }}
    .skill-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
      gap: 10px;
    }}
    .skill-card {{
      border: 1px solid var(--line);
      background: var(--panel);
      border-radius: 8px;
      padding: 12px;
      min-height: 132px;
      display: grid;
      align-content: start;
      gap: 8px;
    }}
    .skill-title {{
      display: flex;
      justify-content: space-between;
      gap: 10px;
      font-size: 14px;
      font-weight: 650;
    }}
    .level {{ color: var(--accent); white-space: nowrap; }}
    .muted {{ color: var(--muted); font-size: 12px; line-height: 1.35; }}
    .bar {{ height: 8px; border-radius: 4px; background: #0f1211; overflow: hidden; }}
    .fill {{ height: 100%; background: var(--accent); }}
    .low .fill {{ background: var(--low); }}
    .mid .fill {{ background: var(--warn); }}
    @media (max-width: 700px) {{
      header {{ padding: 14px; }}
      main {{ padding: 12px; }}
      .skill-grid {{ grid-template-columns: 1fr; }}
    }}
  </style>
</head>
<body>
  <header>
    <h1>CYNIC Skill Matrix</h1>
    <nav class="nav"><a class="pill" href="/">opportunities</a><a class="pill" href="/skills">skills</a></nav>
  </header>
  <main>{sections}</main>
</body>
</html>"""


def render_skill_card(row: sqlite3.Row) -> str:
    level = int(row["level"] or 0)
    confidence = float(row["confidence"] or 0.0)
    evidence = []
    if row["evidence_json"]:
        try:
            evidence = json.loads(row["evidence_json"])
        except json.JSONDecodeError:
            evidence = []
    tone = "low" if level < 2 else "mid" if level < 4 else "high"
    evidence_text = summarize_evidence(evidence)
    return f"""<article class="skill-card {tone}">
  <div class="skill-title">
    <span>{html.escape(row["skill"])}</span>
    <span class="level">L{level}</span>
  </div>
  <div class="bar"><div class="fill" style="width:{level * 20}%"></div></div>
  <div class="muted">{html.escape(row["status"] or "unknown")} · confidence {confidence:.2f}</div>
  <div class="muted">{html.escape(evidence_text)}</div>
</article>"""


def serialize_skill_row(row: sqlite3.Row) -> dict[str, object]:
    evidence: list[object] = []
    judge_verdict: object | None = None
    if row["evidence_json"]:
        try:
            evidence = json.loads(row["evidence_json"])
        except json.JSONDecodeError:
            evidence = []
    if row["judge_verdict_json"]:
        try:
            judge_verdict = json.loads(row["judge_verdict_json"])
        except json.JSONDecodeError:
            judge_verdict = None
    return {
        "category": row["category"],
        "skill": row["skill"],
        "source": row["source"],
        "level": row["level"] or 0,
        "confidence": row["confidence"] or 0,
        "status": row["status"] or "absent",
        "evidence": evidence,
        "judge_verdict": judge_verdict,
        "updated_at": row["updated_at"],
    }


def summarize_evidence(evidence: list[object]) -> str:
    if not evidence:
        return "no local evidence yet"
    labels = []
    for item in evidence[:3]:
        if isinstance(item, dict):
            kind = str(item.get("kind", "evidence"))
            path = str(item.get("path", ""))
            labels.append(f"{kind}: {path}")
        else:
            labels.append(str(item))
    return ", ".join(labels)


def render_card(row: sqlite3.Row) -> str:
    title = html.escape(row["title"])
    url = html.escape(row["url"])
    status = html.escape(row["status"])
    notes = html.escape(row["notes"] or "")
    options = "".join(
        f'<option value="{s}" {"selected" if s == status else ""}>{s}</option>'
        for s in STATUSES
    )
    return f"""<article>
  <section>
    <h2>{title}</h2>
    <div class="meta">
      <span class="score">fit {row["fit_score"]}</span>
      <span>{html.escape(row["category"] or "uncategorized")}</span>
      <span>{html.escape(row["capability"] or "unknown")}</span>
      <span class="reward">{html.escape(row["reward"] or "reward unknown")}</span>
      <span>{status}</span>
      <span>platform {html.escape(row["platform_status"] or "unseen")}</span>
      <span>{html.escape((row["platform_last_seen"] or "")[:16])}</span>
    </div>
    {render_platform_excerpt(row)}
  </section>
  <form class="actions" method="post" action="/update">
    <input type="hidden" name="url" value="{url}">
    <a class="open" href="{url}" target="_blank" rel="noreferrer">Open Earn listing</a>
    <select name="status">{options}</select>
    <textarea name="notes" placeholder="decision notes">{notes}</textarea>
    <button type="submit">Save</button>
  </form>
</article>"""


def render_platform_excerpt(row: sqlite3.Row) -> str:
    excerpt = row["platform_excerpt"] or ""
    if not excerpt:
        return ""
    return (
        '<p style="color:var(--muted);font-size:13px;line-height:1.45;margin:10px 0 0">'
        + html.escape(excerpt[:360])
        + "</p>"
    )


def main() -> None:
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    print(f"organ-discord UI listening on http://{HOST}:{PORT}")
    server.serve_forever()


if __name__ == "__main__":
    main()
