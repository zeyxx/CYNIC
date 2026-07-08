import json
import os
import re
from pathlib import Path
from urllib.error import URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

from organs.contracts.src.surface_contracts import json_loads_or_default
from organs.contracts.src.surface_contracts import json_object_or_default

from ..core.entities import ResolvedSocialSource
from ..core.ports import ISocialSourceResolver
from .context_repository import read_jsonl


TWEET_ID_RE = re.compile(r"/status(?:es)?/(\d+)")
ARTICLE_ID_RE = re.compile(r"/(?:i/)?article/(\d+)")


def extract_tweet_id(url: str) -> str:
    match = TWEET_ID_RE.search(url)
    return match.group(1) if match else ""


def extract_article_id(url: str) -> str:
    match = ARTICLE_ID_RE.search(url)
    return match.group(1) if match else ""


def extract_handle(url: str) -> str:
    parsed = urlparse(url)
    parts = [part for part in parsed.path.split("/") if part]
    if parts[:2] == ["i", "article"]:
        return ""
    return parts[0] if parts else ""


def canonical_article_url(article_id: str) -> str:
    return f"https://x.com/i/article/{article_id}" if article_id else ""


class LocalHermesDatasetResolver(ISocialSourceResolver):
    def __init__(self, dataset_file: Path):
        self.dataset_file = dataset_file

    def resolve(self, url: str) -> ResolvedSocialSource:
        tweet_id = extract_tweet_id(url)
        if not tweet_id:
            raise ValueError(f"not an X status URL: {url}")
        for row in read_jsonl(self.dataset_file):
            if str(row.get("tweet_id") or row.get("id") or "") == tweet_id:
                author = str(row.get("author_screen_name") or row.get("author") or extract_handle(url))
                tags = list(row.get("cashtags") or []) + list(row.get("narratives") or [])
                return ResolvedSocialSource(
                    url=url,
                    source_type="tweet",
                    text=str(row.get("text") or ""),
                    author=author,
                    title=f"Tweet {tweet_id}",
                    tags=tags,
                    notes="Resolved from local Hermes X dataset.",
                    metadata={"resolver": "local-hermes-dataset", "tweet_id": tweet_id, "raw": row},
                )
        raise LookupError(f"tweet {tweet_id} not found in local Hermes X dataset")


class RawCaptureResolver(ISocialSourceResolver):
    def __init__(self, captures_dir: Path):
        self.captures_dir = captures_dir

    def resolve(self, url: str) -> ResolvedSocialSource:
        tweet_id = extract_tweet_id(url)
        if not tweet_id:
            raise ValueError(f"not an X status URL: {url}")
        if not self.captures_dir.exists():
            raise LookupError(f"captures directory not found: {self.captures_dir}")

        for capture_file in sorted(self.captures_dir.glob("*.json"), reverse=True):
            try:
                raw = capture_file.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            if tweet_id not in raw:
                continue
            payload = json_loads_or_default(raw, [])
            if not isinstance(payload, (dict, list)):
                continue
            tweet = _find_tweet_object(payload, tweet_id)
            if tweet:
                return _tweet_object_to_source(tweet, url, capture_file)
        raise LookupError(f"tweet {tweet_id} not found in raw captures")


def _find_tweet_object(obj: object, tweet_id: str) -> dict | None:
    if isinstance(obj, dict):
        if obj.get("rest_id") == tweet_id and isinstance(obj.get("legacy"), dict):
            return obj
        legacy = obj.get("legacy")
        if isinstance(legacy, dict) and legacy.get("id_str") == tweet_id and "retweeted_status_result" in legacy:
            result = legacy.get("retweeted_status_result", {}).get("result")
            if isinstance(result, dict):
                return result
        for value in obj.values():
            found = _find_tweet_object(value, tweet_id)
            if found:
                return found
    elif isinstance(obj, list):
        for value in obj:
            found = _find_tweet_object(value, tweet_id)
            if found:
                return found
    return None


def _tweet_object_to_source(tweet: dict, url: str, capture_file: Path) -> ResolvedSocialSource:
    tweet_id = str(tweet.get("rest_id") or "")
    legacy = tweet.get("legacy") or {}
    user = tweet.get("core", {}).get("user_results", {}).get("result", {})
    user_core = user.get("core") or {}
    user_legacy = user.get("legacy") or {}
    article = (
        tweet.get("article", {})
        .get("article_results", {})
        .get("result", {})
    )
    article_title = str(article.get("title") or "")
    article_preview = str(article.get("preview_text") or "")
    article_id = str(article.get("rest_id") or "")
    text_parts = []
    full_text = str(legacy.get("full_text") or "").strip()
    if full_text:
        text_parts.append(f"Tweet text: {full_text}")
    if article_title:
        text_parts.append(f"Article title: {article_title}")
    if article_preview:
        text_parts.append(f"Article preview: {article_preview}")
    if not text_parts:
        text_parts.append(f"Captured X/Twitter tweet {tweet_id}.")

    metrics = {
        "likes": legacy.get("favorite_count"),
        "retweets": legacy.get("retweet_count"),
        "replies": legacy.get("reply_count"),
        "quotes": legacy.get("quote_count"),
        "bookmarks": legacy.get("bookmark_count"),
        "views": (tweet.get("views") or {}).get("count"),
    }
    tags = ["x-capture"]
    if article_id:
        tags.append("x-article")

    return ResolvedSocialSource(
        url=url,
        source_type="tweet",
        text="\n".join(text_parts),
        author=str(user_core.get("screen_name") or extract_handle(url)),
        title=article_title or f"Tweet {tweet_id}",
        tags=tags,
        notes="Resolved from raw Hermes X capture.",
        metadata={
            "resolver": "raw-capture",
            "tweet_id": tweet_id,
            "article_id": article_id,
            "article_url": f"https://x.com/i/article/{article_id}" if article_id else "",
            "article_extraction_status": "preview_only" if article_id else "not_article",
            "article_full_text_available": False,
            "article_title": article_title,
            "article_preview": article_preview,
            "capture_file": str(capture_file),
            "author_display_name": str(user_core.get("name") or ""),
            "author_followers_count": user_legacy.get("followers_count"),
            "metrics": metrics,
        },
    )


class PublicEmbedResolver(ISocialSourceResolver):
    def resolve(self, url: str) -> ResolvedSocialSource:
        tweet_id = extract_tweet_id(url)
        handle = extract_handle(url)
        if not tweet_id or not handle:
            raise ValueError(f"not an X status URL: {url}")

        for endpoint in (
            f"https://api.fxtwitter.com/{handle}/status/{tweet_id}",
            f"https://api.vxtwitter.com/{handle}/status/{tweet_id}",
        ):
            try:
                request = Request(endpoint, headers={"User-Agent": "CYNIC-organ-twitter/1.0"})
                with urlopen(request, timeout=8) as response:
                    payload = json_object_or_default(response.read().decode("utf-8"))
                tweet = payload.get("tweet") or payload
                text = str(tweet.get("text") or tweet.get("content") or "")
                author = str(tweet.get("author_screen_name") or tweet.get("author") or handle)
                if text:
                    return ResolvedSocialSource(
                        url=url,
                        source_type="tweet",
                        text=text,
                        author=author,
                        title=f"Tweet {tweet_id}",
                        tags=[],
                        notes=f"Resolved through {urlparse(endpoint).netloc}.",
                        metadata={"resolver": "public-embed", "tweet_id": tweet_id, "endpoint": endpoint},
                    )
            except (OSError, URLError, json.JSONDecodeError, KeyError, TimeoutError):
                continue
        raise LookupError(f"public X mirrors did not resolve tweet {tweet_id}")


class BrowserHubRenderResolver(ISocialSourceResolver):
    def __init__(self, hub_url: str | None = None):
        self.hub_url = (hub_url or os.environ.get("ORGAN_BROWSER_HUB_URL") or "http://127.0.0.1:40770").rstrip("/")

    def resolve(self, url: str) -> ResolvedSocialSource:
        tweet_id = extract_tweet_id(url)
        handle = extract_handle(url)
        if not tweet_id:
            raise ValueError(f"not an X status URL: {url}")

        endpoint = f"{self.hub_url}/render?url={url}&wait=8000"
        try:
            request = Request(endpoint, headers={"User-Agent": "CYNIC-organ-twitter/1.0"})
            with urlopen(request, timeout=15) as response:
                payload = json_object_or_default(response.read().decode("utf-8"))
        except (OSError, URLError, json.JSONDecodeError, TimeoutError) as exc:
            raise LookupError(f"browser hub render failed: {exc}") from exc

        if payload.get("error"):
            raise LookupError(f"browser hub render error: {payload.get('error')}")

        text = str(payload.get("markdown") or payload.get("text") or payload.get("content") or "").strip()
        text = _extract_likely_tweet_text(text, handle)
        if not text:
            raise LookupError("browser hub rendered no tweet text")
        return ResolvedSocialSource(
            url=url,
            source_type="tweet",
            text=text,
            author=handle,
            title=f"Tweet {tweet_id}",
            tags=[],
            notes="Resolved through local authenticated Browser Hub render.",
            metadata={"resolver": "browser-hub-render", "tweet_id": tweet_id, "hub_url": self.hub_url},
        )


class AuthenticatedBrowserCdpArticleResolver(ISocialSourceResolver):
    def __init__(self, cdp_url: str | None = None, wait_ms: int = 12000):
        self.cdp_url = (cdp_url or os.environ.get("ORGAN_BROWSER_CDP_URL") or "http://127.0.0.1:40769").rstrip("/")
        self.wait_ms = wait_ms

    def resolve(self, url: str) -> ResolvedSocialSource:
        tweet_id = extract_tweet_id(url)
        article_id = extract_article_id(url)
        if not tweet_id and not article_id:
            raise ValueError(f"not an X status/article URL: {url}")

        try:
            from playwright.sync_api import Error as PlaywrightError
            from playwright.sync_api import TimeoutError as PlaywrightTimeoutError
            from playwright.sync_api import sync_playwright
        except ImportError as exc:
            raise LookupError("playwright is not installed for authenticated Browser CDP extraction") from exc

        try:
            with sync_playwright() as playwright:
                try:
                    browser = playwright.chromium.connect_over_cdp(self.cdp_url)
                    context = browser.contexts[0] if browser.contexts else browser.new_context()
                except Exception:
                    from organs.twitter.src.adapters.hermes_paths import HERMES_X_DIR
                    user_data_dir = HERMES_X_DIR / "chrome_profile"
                    context = playwright.chromium.launch_persistent_context(
                        user_data_dir=str(user_data_dir),
                        headless=True,
                        args=["--disable-blink-features=AutomationControlled"]
                    )
                
                page = context.new_page()
                try:
                    page.goto(url, wait_until="domcontentloaded", timeout=30000)
                    page.wait_for_timeout(self.wait_ms)
                    rendered_text = page.locator("body").inner_text(timeout=5000)
                    final_url = page.url
                finally:
                    page.close()
                    try:
                        context.close()
                    except Exception:
                        pass
        except (PlaywrightError, PlaywrightTimeoutError, OSError) as exc:
            raise LookupError(f"authenticated browser CDP or persistent headless extraction failed: {exc}") from exc

        article = extract_x_article_from_visible_text(rendered_text, source_url=final_url or url)
        if not article:
            raise LookupError("authenticated browser CDP did not expose a full X article")

        metadata = {
            "resolver": "authenticated-browser-cdp-article",
            "tweet_id": tweet_id,
            "article_id": article["article_id"] or article_id or tweet_id,
            "article_url": article["article_url"] or canonical_article_url(article_id or tweet_id),
            "article_extraction_status": "full_text",
            "article_full_text_available": True,
            "extraction_method": "browser-hub-authenticated-cdp",
            "cdp_url": self.cdp_url,
            "final_url": final_url,
        }
        return ResolvedSocialSource(
            url=metadata["article_url"] or url,
            source_type="article",
            text=article["text"],
            author=article["author"],
            title=article["title"],
            tags=["x-article", "full-text"],
            notes="Resolved from authenticated X browser session through Browser CDP.",
            metadata=metadata,
        )


def extract_x_article_from_visible_text(rendered: str, source_url: str = "") -> dict[str, str] | None:
    lines = [line.strip() for line in rendered.splitlines() if line.strip()]
    if not lines:
        return None

    title_index = _find_article_title_index(lines)
    if title_index is None:
        return None
    title = lines[title_index]

    body_start = _find_article_body_start(lines, title_index)
    body_end = _find_article_body_end(lines, body_start)
    body_lines = lines[body_start:body_end]
    if not body_lines or len("\n".join(body_lines)) < 1000:
        return None

    author = _extract_visible_article_author(lines, title_index)
    published = _extract_visible_article_date(lines, title_index)
    article_id = extract_article_id(source_url) or extract_tweet_id(source_url)
    article_url = canonical_article_url(article_id)

    header = [title]
    if author:
        header.append(f"Author: {author}")
    if published:
        header.append(f"Published: {published}")
    if article_url:
        header.append(f"Source: {article_url}")
    header.append("")

    return {
        "title": title,
        "author": author.lstrip("@"),
        "published": published,
        "article_id": article_id,
        "article_url": article_url,
        "text": "\n".join(header + body_lines),
    }


def _find_article_title_index(lines: list[str]) -> int | None:
    blocked = {
        "post",
        "article",
        "happening now.",
        "continue with phone",
        "continue with apple",
        "email or username",
        "continue",
        "to view keyboard shortcuts, press question mark",
        "view keyboard shortcuts",
    }
    for index, line in enumerate(lines[:40]):
        lowered = line.lower()
        if lowered in blocked:
            continue
        if len(line) >= 20 and not line.startswith("@") and not line.startswith("http"):
            return index
    return None


def _find_article_body_start(lines: list[str], title_index: int) -> int:
    for index in range(title_index + 1, min(len(lines), title_index + 20)):
        line = lines[index]
        lowered = line.lower()
        if lowered.startswith(("you ", "the ", "this ", "series ", "in ", "a ", "an ", "if ", "when ", "what ")):
            return index
    return min(title_index + 1, len(lines))


def _find_article_body_end(lines: list[str], body_start: int) -> int:
    terminal_markers = {
        "want to publish your own article?",
        "upgrade to premium",
        "read more",
    }
    for index in range(body_start, len(lines)):
        if lines[index].lower() in terminal_markers:
            return index
    return len(lines)


def _extract_visible_article_author(lines: list[str], title_index: int) -> str:
    for index in range(title_index + 1, min(len(lines), title_index + 8)):
        line = lines[index]
        if line.startswith("@"):
            previous = lines[index - 1] if index > 0 else ""
            return f"{previous} ({line})" if previous and not previous.startswith("@") else line
    return ""


def _extract_visible_article_date(lines: list[str], title_index: int) -> str:
    for index in range(title_index + 1, min(len(lines), title_index + 12)):
        line = lines[index]
        if re.search(r"\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b", line):
            return line
    return ""


def _extract_likely_tweet_text(rendered: str, handle: str) -> str:
    lines = [line.strip() for line in rendered.splitlines() if line.strip()]
    if not lines:
        return ""
    handle_lower = handle.lower().lstrip("@")
    candidates: list[str] = []
    for index, line in enumerate(lines):
        low = line.lower()
        if handle_lower and handle_lower in low:
            candidates.extend(lines[index + 1:index + 6])
    candidates.extend(lines)
    blocked = {"x", "home", "search", "notifications", "messages", "profile", "log in", "sign up"}
    for candidate in candidates:
        plain = re.sub(r"\s+", " ", candidate).strip()
        if len(plain) >= 20 and plain.lower() not in blocked and "/status/" not in plain:
            return plain[:2000]
    return ""


class FallbackUnresolvedResolver(ISocialSourceResolver):
    def resolve(self, url: str) -> ResolvedSocialSource:
        handle = extract_handle(url)
        tweet_id = extract_tweet_id(url)
        return ResolvedSocialSource(
            url=url,
            source_type="tweet" if tweet_id else "article",
            text=f"Unresolved X/Twitter source URL: {url}",
            author=handle,
            title=f"Tweet {tweet_id}" if tweet_id else url,
            tags=["needs-x-resolution"],
            notes="Resolver could not retrieve text from local dataset or public mirrors.",
            metadata={"resolver": "fallback-unresolved", "tweet_id": tweet_id},
        )


class CompositeSocialSourceResolver(ISocialSourceResolver):
    def __init__(self, resolvers: list[ISocialSourceResolver]):
        self.resolvers = resolvers

    def resolve(self, url: str) -> ResolvedSocialSource:
        errors: list[str] = []
        for resolver in self.resolvers:
            try:
                return resolver.resolve(url)
            except (LookupError, ValueError, OSError, URLError) as exc:
                errors.append(f"{resolver.__class__.__name__}: {exc}")
        unresolved = FallbackUnresolvedResolver().resolve(url)
        unresolved.metadata["resolver_errors"] = errors
        return unresolved
