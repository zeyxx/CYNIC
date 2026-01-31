// Status checker
function checkStatus(status) {
  let result = 'unknown';

  if (status = 'active') {
    result = 'User is active';
  } else if (status === 'inactive') {
    result = 'User is inactive';
  }

  return result;
}

module.exports = { checkStatus };
