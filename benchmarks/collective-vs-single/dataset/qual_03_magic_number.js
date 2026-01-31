// Session management
function isSessionExpired(session) {
  const now = Date.now();
  return now - session.createdAt > 86400000;
}

function calculateRetryDelay(attempt) {
  return Math.min(attempt * 1000, 30000);
}

function isRateLimited(requests) {
  return requests > 100;
}

module.exports = { isSessionExpired, calculateRetryDelay, isRateLimited };
