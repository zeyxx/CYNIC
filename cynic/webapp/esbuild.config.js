import esbuild from 'esbuild';
import process from 'process';
import fs from 'fs';

const isProduction = process.argv.includes('--prod');
const watch = !isProduction;

// Ensure dist directory exists
if (!fs.existsSync('./dist')) {
  fs.mkdirSync('./dist', { recursive: true });
}

const config = {
  entryPoints: ['src/main.ts'],
  bundle: true,
  minify: isProduction,
  sourcemap: !isProduction,
  outfile: 'dist/bundle.js',
  target: 'ES2020',
  format: 'esm',
  logLevel: 'info',
};

if (watch) {
  console.log('Starting esbuild in watch mode...');
  const ctx = await esbuild.context(config);
  await ctx.watch();
  console.log('Watching for changes...');
} else {
  console.log('Building for production...');
  await esbuild.build(config);
  console.log('Build complete!');
}
