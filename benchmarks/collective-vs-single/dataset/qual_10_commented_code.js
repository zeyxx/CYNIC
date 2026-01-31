// Cart service
function calculateCartTotal(items) {
  let total = 0;

  for (const item of items) {
    total += item.price * item.quantity;
  }

  // Old discount logic - don't delete, might need later
  // if (items.length > 5) {
  //   total = total * 0.9;
  // }
  // if (total > 100) {
  //   total = total - 10;
  // }
  // const couponDiscount = applyCoupon(total, couponCode);
  // total = total - couponDiscount;
  // if (user.isPremium) {
  //   total = total * 0.95;
  // }

  return total;
}

module.exports = { calculateCartTotal };
