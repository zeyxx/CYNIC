// Calculator API endpoint
function calculate(req, res) {
  const { expression } = req.body;

  try {
    const result = eval(expression);
    res.json({ result });
  } catch (error) {
    res.status(400).json({ error: 'Invalid expression' });
  }
}

module.exports = { calculate };
