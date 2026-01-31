// Access control
function canAccess(user, resource) {
  // User must be admin AND (owner OR public resource)
  if (user.isAdmin && user.id === resource.ownerId && resource.isPublic) {
    return true;
  }
  return false;
}

module.exports = { canAccess };
