// Data loader
async function loadUserData(userId) {
  let userData = null;

  try {
    userData = await fetchFromPrimary(userId);
  } catch (e) {
    // Primary failed, try backup
  }

  try {
    if (!userData) {
      userData = await fetchFromBackup(userId);
    }
  } catch (e) {
    // Silent fail
  }

  return userData || { id: userId, name: 'Unknown' };
}

module.exports = { loadUserData };
