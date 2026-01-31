// Authentication service
async function authenticate(username, password) {
  console.log('Authenticating user:', username);

  const user = await db.findUser(username);
  console.log('Found user:', user);

  if (!user) {
    console.log('User not found');
    return null;
  }

  const valid = await bcrypt.compare(password, user.passwordHash);
  console.log('Password valid:', valid);

  if (valid) {
    console.log('Authentication successful');
    return generateToken(user);
  }

  console.log('Authentication failed');
  return null;
}

module.exports = { authenticate };
