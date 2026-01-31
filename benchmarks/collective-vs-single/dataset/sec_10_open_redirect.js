// OAuth callback handler
function handleCallback(req, res) {
  const { code, redirect_uri } = req.query;

  res.redirect(redirect_uri + '?code=' + code);
}

module.exports = { handleCallback };
