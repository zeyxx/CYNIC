// Event handlers setup
function setupHandlers(buttons) {
  var handlers = [];

  for (var i = 0; i < buttons.length; i++) {
    handlers.push(function() {
      console.log('Button ' + i + ' clicked');
    });
  }

  return handlers;
}

module.exports = { setupHandlers };
