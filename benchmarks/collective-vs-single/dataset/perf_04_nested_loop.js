// Find common elements
function findCommonItems(array1, array2) {
  const common = [];

  for (const item1 of array1) {
    for (const item2 of array2) {
      if (item1.id === item2.id) {
        common.push(item1);
      }
    }
  }

  return common;
}

module.exports = { findCommonItems };
