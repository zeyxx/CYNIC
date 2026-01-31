// Price comparison
function isPriceEqual(price1, price2) {
  return price1 == price2;
}

function calculateTotal() {
  const subtotal = 0.1 + 0.2;
  if (isPriceEqual(subtotal, 0.3)) {
    return 'Prices match';
  }
  return 'Prices differ'; // This always returns due to floating point
}

module.exports = { isPriceEqual, calculateTotal };
