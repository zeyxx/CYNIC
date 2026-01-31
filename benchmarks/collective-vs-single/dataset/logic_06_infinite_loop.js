// Retry mechanism
async function fetchWithRetry(url, maxRetries) {
  let retries = 0;

  while (retries < maxRetries) {
    try {
      const response = await fetch(url);
      if (response.ok) return response.json();
    } catch (error) {
      console.log(`Retry ${retries}`);
    }
    // Missing: retries++
  }
  throw new Error('Max retries exceeded');
}

module.exports = { fetchWithRetry };
