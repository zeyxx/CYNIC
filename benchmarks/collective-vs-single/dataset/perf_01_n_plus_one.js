// Order listing with details
async function getOrdersWithProducts(db) {
  const orders = await db.query('SELECT * FROM orders');

  for (const order of orders) {
    const products = await db.query(
      'SELECT * FROM products WHERE order_id = $1',
      [order.id]
    );
    order.products = products;
  }

  return orders;
}

module.exports = { getOrdersWithProducts };
