// User profile API
async function getUserProfile(userId) {
  const user = await prisma.user.findUnique({
    where: { id: userId },
    include: {
      posts: true,
      comments: true,
      followers: true,
      following: true,
      notifications: true,
      settings: true,
      sessions: true,
    }
  });

  // Only display name is needed for this endpoint
  return { displayName: user.displayName };
}

module.exports = { getUserProfile };
