// Email validation
function validateEmail(email) {
  const regex = /^([a-zA-Z0-9_\.\-])+\@(([a-zA-Z0-9\-])+\.)+([a-zA-Z0-9]{2,4})+$/;
  return regex.test(email);
}

function processSignup(req, res) {
  const { email, password } = req.body;
  if (!validateEmail(email)) {
    return res.status(400).json({ error: 'Invalid email' });
  }
  // Continue signup...
  res.json({ success: true });
}

module.exports = { validateEmail, processSignup };
