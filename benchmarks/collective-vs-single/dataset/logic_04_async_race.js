// Counter service
let counter = 0;

async function incrementCounter() {
  const current = counter;
  await new Promise(resolve => setTimeout(resolve, 10));
  counter = current + 1;
  return counter;
}

async function getAndIncrement() {
  return await incrementCounter();
}

module.exports = { incrementCounter, getAndIncrement };
