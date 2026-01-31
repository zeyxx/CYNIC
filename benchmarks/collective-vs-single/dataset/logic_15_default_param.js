// Configuration builder
function createConfig(options = { plugins: [] }) {
  options.plugins.push('default-plugin');
  return options;
}

function setup() {
  const config1 = createConfig();
  const config2 = createConfig();
  // config2.plugins now has ['default-plugin', 'default-plugin']
  return { config1, config2 };
}

module.exports = { createConfig, setup };
