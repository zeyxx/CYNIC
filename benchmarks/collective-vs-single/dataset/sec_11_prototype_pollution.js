// Configuration merger utility
function deepMerge(target, source) {
  for (const key in source) {
    if (source[key] && typeof source[key] === 'object') {
      if (!target[key]) target[key] = {};
      deepMerge(target[key], source[key]);
    } else {
      target[key] = source[key];
    }
  }
  return target;
}

function updateConfig(req, res) {
  const config = deepMerge({}, req.body);
  res.json(config);
}

module.exports = { deepMerge, updateConfig };
