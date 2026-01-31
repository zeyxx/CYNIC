// Form validator
function validateForm(data) {
  if (!data.email) {
    return { valid: false, error: 'Email required' };
  }

  if (!data.password) {
    return { valid: false, error: 'Password required' };
    console.log('Password validation failed'); // Unreachable
  }

  return { valid: true };
}

module.exports = { validateForm };
