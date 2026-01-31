// Comment display component
function renderComment(req, res) {
  const { comment } = req.query;

  res.send(`
    <div class="comment">
      <p>${comment}</p>
    </div>
  `);
}

module.exports = { renderComment };
