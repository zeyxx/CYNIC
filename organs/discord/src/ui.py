#!/usr/bin/env python3
"""Local UI for the organ-discord opportunity queue."""
from __future__ import annotations

import html
import os
import sqlite3
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

DB_PATH = Path(os.environ.get("DISCORD_DB_PATH", "~/.cynic/organs/discord/messages.db")).expanduser()
HOST = os.environ.get("DISCORD_UI_HOST", "127.0.0.1")
PORT = int(os.environ.get("DISCORD_UI_PORT", "8891"))

STATUSES = ("new", "reviewing", "applied", "won", "lost", "ignored")


def connect() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn


def ensure_notes_column(conn: sqlite3.Connection) -> None:
    cols = {row[1] for row in conn.execute("PRAGMA table_info(opportunities)")}
    if "notes" not in cols:
        conn.execute("ALTER TABLE opportunities ADD COLUMN notes TEXT NOT NULL DEFAULT ''")
        conn.commit()


def rows(status: str | None = None) -> list[sqlite3.Row]:
    conn = connect()
    ensure_notes_column(conn)
    params: list[str] = []
    where = ""
    if status and status in STATUSES:
        where = "WHERE status = ?"
        params.append(status)
    data = conn.execute(
        f"""SELECT title, category, reward, reward_usd, url, fit_score, status,
                  first_seen, last_seen, notes
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
    ensure_notes_column(conn)
    result = {status: 0 for status in STATUSES}
    for status, count in conn.execute(
        "SELECT status, count(*) FROM opportunities GROUP BY status"
    ).fetchall():
        result[status] = count
    conn.close()
    return result


def update(url: str, status: str, notes: str | None = None) -> None:
    if status not in STATUSES:
        return
    conn = connect()
    ensure_notes_column(conn)
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
        if parsed.path != "/":
            self.send_error(404)
            return
        status = parse_qs(parsed.query).get("status", [None])[0]
        self.respond(render(status))

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

    def log_message(self, fmt: str, *args: object) -> None:
        return


def render(active_status: str | None) -> str:
    all_rows = rows(active_status)
    count_map = counts()
    nav = " ".join(
        f'<a class="pill" href="/?status={status}">{status} {count_map.get(status, 0)}</a>'
        for status in STATUSES
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
    <nav class="nav"><a class="pill" href="/">all</a>{nav}</nav>
  </header>
  <main>{cards or '<p>No opportunities yet.</p>'}</main>
</body>
</html>"""


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
      <span class="reward">{html.escape(row["reward"] or "reward unknown")}</span>
      <span>{status}</span>
    </div>
  </section>
  <form class="actions" method="post" action="/update">
    <input type="hidden" name="url" value="{url}">
    <a class="open" href="{url}" target="_blank" rel="noreferrer">Open Earn listing</a>
    <select name="status">{options}</select>
    <textarea name="notes" placeholder="decision notes">{notes}</textarea>
    <button type="submit">Save</button>
  </form>
</article>"""


def main() -> None:
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    print(f"organ-discord UI listening on http://{HOST}:{PORT}")
    server.serve_forever()


if __name__ == "__main__":
    main()
