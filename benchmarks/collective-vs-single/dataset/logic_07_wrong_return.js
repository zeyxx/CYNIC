// Price calculator
function calculateDiscount(price, discountPercent) {
  const discount = price * (discountPercent / 100);
  const finalPrice = price - discount;

  return discount; // Should return finalPrice
}

function applyDiscount(cartTotal) {
  return calculateDiscount(cartTotal, 15);
}

module.exports = { calculateDiscount, applyDiscount };
