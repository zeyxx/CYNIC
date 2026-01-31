// User profile handler
function getDisplayName(user) {
  // Get the name to display
  const name = user.profile.displayName;

  if (user && user.profile) {
    return name || user.username;
  }
  return 'Anonymous';
}

module.exports = { getDisplayName };
