// Permission checker
function canPerformAction(user, action, resource) {
  if (user) {
    if (user.active) {
      if (user.roles) {
        if (user.roles.includes('admin')) {
          return true;
        } else {
          if (resource.permissions) {
            if (resource.permissions[action]) {
              if (resource.permissions[action].includes(user.id)) {
                return true;
              }
            }
          }
        }
      }
    }
  }
  return false;
}

module.exports = { canPerformAction };
