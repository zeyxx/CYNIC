// Array utilities
function addItem(array, item) {
  const copy = array;
  copy.push(item);
  return copy;
}

function processOrder(items) {
  const original = ['base'];
  const updated = addItem(original, 'extra');
  // original is now mutated too
  return { original, updated };
}

module.exports = { addItem, processOrder };
