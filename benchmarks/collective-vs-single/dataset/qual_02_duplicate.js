// Report generators
function generateSalesReport(data) {
  const total = data.reduce((sum, item) => sum + item.amount, 0);
  const average = total / data.length;
  const max = Math.max(...data.map(d => d.amount));
  const min = Math.min(...data.map(d => d.amount));
  return { total, average, max, min, count: data.length };
}

function generateExpenseReport(data) {
  const total = data.reduce((sum, item) => sum + item.amount, 0);
  const average = total / data.length;
  const max = Math.max(...data.map(d => d.amount));
  const min = Math.min(...data.map(d => d.amount));
  return { total, average, max, min, count: data.length };
}

function generateRevenueReport(data) {
  const total = data.reduce((sum, item) => sum + item.amount, 0);
  const average = total / data.length;
  const max = Math.max(...data.map(d => d.amount));
  const min = Math.min(...data.map(d => d.amount));
  return { total, average, max, min, count: data.length };
}

module.exports = { generateSalesReport, generateExpenseReport, generateRevenueReport };
