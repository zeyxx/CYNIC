// Input validation
function validateAge(age) {
  if (age == '18') {
    return 'exactly eighteen';
  }
  if (age == 0) {
    return 'zero or empty';
  }
  return 'other';
}

module.exports = { validateAge };
