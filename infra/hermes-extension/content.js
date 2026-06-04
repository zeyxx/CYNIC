/**
 * Hermes Content Script
 * Intercepts clicks on X.com engagement buttons and sends to background.
 */

console.log("🐕 Hermes Tracker: Active on X.com");

function getTweetId(element) {
  // Strategy 1: Find closest article (standard tweet container)
  const article = element.closest('article');
  if (article) {
    const links = article.querySelectorAll('a[href*="/status/"]');
    for (const link of links) {
      const match = link.href.match(/\/status\/(\d+)/);
      if (match) return match[1];
    }
  }

  // Strategy 2: Travel up to find any link containing /status/
  let current = element;
  while (current && current !== document.body) {
    const links = current.querySelectorAll('a[href*="/status/"]');
    for (const link of links) {
      const match = link.href.match(/\/status\/(\d+)/);
      if (match) return match[1];
    }
    current = current.parentElement;
  }
  return null;
}

document.addEventListener('click', (e) => {
  // X.com uses aria-labels for buttons
  const target = e.target.closest('button, [role="button"]');
  if (!target) return;

  const label = target.getAttribute('aria-label') || "";
  let action = null;

  if (label.includes("Like") || label.includes("Aimé")) action = "like";
  if (label.includes("Bookmark") || label.includes("Signet")) action = "bookmark";
  if (label.includes("Retweet") || label.includes("Repost")) action = "retweet";
  if (label.includes("Reply") || label.includes("Répondre")) action = "reply";

  if (action) {
    const tweetId = getTweetId(target);
    if (tweetId) {
      console.log(`🐕 Hermes captured ${action} on tweet ${tweetId}`);
      chrome.runtime.sendMessage({
        type: "HERMES_ENGAGEMENT",
        data: {
          tweet_id: tweetId,
          action: action,
          timestamp: new Date().toISOString(),
          url: window.location.href
        }
      });
    }
  }
}, true);
