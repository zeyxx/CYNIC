// API handlers
function getUserProfile(userId) {
  return db.findUser(userId);
}

function update_user_settings(user_id, settings) {
  return db.updateSettings(user_id, settings);
}

function DeleteUserAccount(UserId) {
  return db.deleteUser(UserId);
}

const get_user_preferences = (user_id) => {
  return db.getPreferences(user_id);
};

module.exports = {
  getUserProfile,
  update_user_settings,
  DeleteUserAccount,
  get_user_preferences
};
