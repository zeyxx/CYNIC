// Payment service
const API_KEY = 'sk_test_FAKE_KEY_FOR_TESTING_1234567890abc';

async function processPayment(amount, cardToken) {
  const response = await fetch('https://api.stripe.com/v1/charges', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${API_KEY}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ amount, source: cardToken })
  });
  return response.json();
}

module.exports = { processPayment };
