// Array processor
function processItems(items) {
  const results = [];
  for (let i = 0; i <= items.length; i++) {
    results.push(items[i].toUpperCase());
  }
  return results;
}

module.exports = { processItems };
