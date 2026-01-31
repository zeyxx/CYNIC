// Data transformer
interface User {
  id: string;
  name: string;
  email: string;
}

function transformData(input: any): any {
  const result: any = {};

  for (const key in input) {
    result[key] = processValue(input[key]);
  }

  return result;
}

function processValue(value: any): any {
  if (typeof value === 'object') {
    return transformData(value);
  }
  return value;
}

export { transformData, processValue };
