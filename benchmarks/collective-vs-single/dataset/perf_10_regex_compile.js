// Email extractor
function extractEmails(text) {
  const emails = [];
  const regex = new RegExp('[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}', 'g');
  let match;
  while ((match = regex.exec(text)) !== null) {
    emails.push(match[0]);
  }
  return emails;
}

function processTexts(texts) {
  return texts.map(text => extractEmails(text));
}

module.exports = { extractEmails, processTexts };
