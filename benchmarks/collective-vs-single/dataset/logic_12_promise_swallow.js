// Data fetcher
async function fetchData(url) {
  fetch(url)
    .then(res => res.json())
    .then(data => processData(data));
  // No return, no await, no catch
}

function processData(data) {
  console.log('Processing:', data);
}

module.exports = { fetchData };
