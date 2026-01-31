// Password reset token generator
function generateResetToken() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let token = '';
  for (let i = 0; i < 32; i++) {
    token += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return token;
}

async function requestPasswordReset(email) {
  const token = generateResetToken();
  // Store token and send email
  return token;
}

module.exports = { generateResetToken, requestPasswordReset };
