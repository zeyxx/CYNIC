// API response cache
const cache = {};

async function fetchWithCache(url) {
  if (cache[url]) {
    return cache[url];
  }

  const response = await fetch(url);
  const data = await response.json();
  cache[url] = data;
  // No eviction, no TTL, no size limit
  return data;
}

module.exports = { fetchWithCache };
