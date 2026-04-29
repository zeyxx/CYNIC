const sharp = require('sharp');
const fs = require('fs');

function esc(str) {
  if (!str) return '';
  return String(str).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

const token = {
    name: 'Solana Diamond',
    symbol: 'DIAMOND',
    price: 0.0004567,
    market_cap: 456000,
    price_change_24h: 12.45,
    conviction_1m: 82,
    holder_count: 5432
};
const mint = '9zB5wRarXMj86MymwLumSKA1Dx35zPqqKfcZtK1Spump';

function generateTokenSvg(token, mint, theme = 'default') {
  const name = esc(token?.name || token?.symbol || 'Unknown Token');
  const symbol = esc(token?.symbol || '');
  const price = '$0.000456';
  const mcap = '$456K';
  const change = token?.price_change_24h ?? null;
  const conviction = token?.conviction_1m ?? null;
  const holders = token?.holder_count || null;

  const changeStr = change !== null ? `${change >= 0 ? '+' : ''}${change.toFixed(2)}%` : '--';
  const convictionStr = conviction !== null ? `${Math.round(conviction)}%` : '--';
  const holdersStr = holders ? holders.toLocaleString() : '--';

  const displayName = name.length > 24 ? name.slice(0, 22) + '..' : name;
  const mintShort = mint.slice(0, 6) + '...' + mint.slice(-4);
  const convBarWidth = conviction !== null ? Math.min(100, Math.max(0, Math.round(conviction))) : 0;

  let bg = '#09090b';
  let accent = '#ff5722';
  let secondary = '#ff8c00';
  let textPrimary = '#f0f0f2';
  let textSecondary = '#6b6b74';
  let glowColor = 'rgba(255,87,34,0.04)';

  if (theme === 'cyber') {
      bg = '#050505';
      accent = '#00f2ff'; // Cyan
      secondary = '#ff00ff'; // Magenta
      textPrimary = '#ffffff';
      glowColor = 'rgba(0,242,255,0.08)';
  } else if (theme === 'emerald') {
      bg = '#060a09';
      accent = '#10b981';
      secondary = '#34d399';
      glowColor = 'rgba(16,185,129,0.08)';
  }

  const changeColor = change !== null ? (change >= 0 ? '#10b981' : '#ef4444') : '#a0a0a8';
  const convBarColor = convBarWidth >= 50 ? accent : convBarWidth >= 20 ? secondary : '#3a3a42';

  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <linearGradient id="bgGrad" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="${bg}"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="accentGrad" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0%" stop-color="${accent}"/>
      <stop offset="100%" stop-color="${secondary}"/>
    </linearGradient>
  </defs>

  <rect width="1200" height="630" fill="url(#bgGrad)"/>
  <ellipse cx="600" cy="0" rx="700" ry="250" fill="${glowColor}"/>
  
  <text x="52" y="60" font-family="sans-serif" font-size="24" font-weight="900" fill="${textPrimary}">Cult<tspan fill="${accent}">Screener</tspan></text>
  
  <text x="52" y="160" font-family="sans-serif" font-size="60" font-weight="800" fill="${textPrimary}">${displayName}</text>
  <text x="52" y="200" font-family="monospace" font-size="20" fill="${textSecondary}">${symbol} · ${mintShort}</text>

  <text x="52" y="300" font-family="sans-serif" font-size="80" font-weight="800" fill="${textPrimary}">${price}</text>
  
  <rect x="52" y="330" width="180" height="40" rx="10" fill="rgba(255,255,255,0.05)"/>
  <text x="65" y="357" font-family="sans-serif" font-size="20" font-weight="700" fill="${changeColor}">${changeStr} 24h</text>

  <text x="620" y="420" font-family="sans-serif" font-size="14" font-weight="600" fill="${accent}" letter-spacing="2">DIAMOND HANDS CONVICTION</text>
  <text x="620" y="460" font-family="sans-serif" font-size="40" font-weight="800" fill="${textPrimary}">${convictionStr}</text>
  
  <rect x="620" y="480" width="530" height="12" rx="6" fill="rgba(255,255,255,0.05)"/>
  <rect x="620" y="480" width="${Math.round(convBarWidth * 5.3)}" height="12" rx="6" fill="url(#accentGrad)"/>

  <line x1="40" y1="560" x2="1160" y2="560" stroke="rgba(255,255,255,0.05)" stroke-width="1"/>
  <text x="52" y="595" font-family="sans-serif" font-size="18" font-weight="600" fill="${textSecondary}">cultscreener.com</text>
</svg>`;
}

async function run() {
    const themes = ['default', 'cyber', 'emerald'];
    for (const theme of themes) {
        const svg = generateTokenSvg(token, mint, theme);
        await sharp(Buffer.from(svg))
          .png()
          .toFile(`theme-${theme}.png`);
        console.log(`Generated: theme-${theme}.png`);
    }
}

run();
