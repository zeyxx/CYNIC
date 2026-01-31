// User service
function getUser(id) {
  return db.users.findById(id);
}

function updateUser(id, data) {
  return db.users.update(id, data);
}

function deleteUser(id) {
  return db.users.delete(id);
}

function legacyUserMigration(users) {
  return users.map(u => ({ ...u, migrated: true }));
}

module.exports = { getUser, updateUser, deleteUser };
