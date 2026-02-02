#!/usr/bin/env node
/**
 * CYNIC X Vision Setup Script
 *
 * Automatically installs CA certificate and configures proxy for X/Twitter capture.
 *
 * Usage:
 *   node scripts/setup-x-vision.js [--install|--uninstall|--status]
 *
 * "Your data, your device, your choice" - κυνικός
 */

import { execSync, spawn } from 'child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, copyFileSync } from 'fs';
import { join, dirname } from 'path';
import { homedir, platform } from 'os';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Configuration
const PROXY_PORT = process.env.CYNIC_X_PROXY_PORT || 8888;
const CERT_DIR = process.env.CYNIC_X_PROXY_CERT_PATH || join(homedir(), '.cynic', 'x-proxy-certs');
const CA_CERT_NAME = 'ca.pem';
const CA_KEY_NAME = 'ca-key.pem';

// Colors for output
const colors = {
  reset: '\x1b[0m',
  cyan: '\x1b[36m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  dim: '\x1b[2m',
};

function log(msg, color = 'reset') {
  console.log(`${colors[color]}${msg}${colors.reset}`);
}

function logStep(step, msg) {
  console.log(`${colors.cyan}[${step}]${colors.reset} ${msg}`);
}

function logSuccess(msg) {
  console.log(`${colors.green}✓${colors.reset} ${msg}`);
}

function logWarning(msg) {
  console.log(`${colors.yellow}⚠${colors.reset} ${msg}`);
}

function logError(msg) {
  console.log(`${colors.red}✗${colors.reset} ${msg}`);
}

/**
 * Generate self-signed CA certificate using OpenSSL
 */
function generateCACert() {
  logStep('1/4', 'Generating CA certificate...');

  // Ensure directory exists
  if (!existsSync(CERT_DIR)) {
    mkdirSync(CERT_DIR, { recursive: true });
  }

  const caKeyPath = join(CERT_DIR, CA_KEY_NAME);
  const caCertPath = join(CERT_DIR, CA_CERT_NAME);

  // Check if already exists
  if (existsSync(caCertPath) && existsSync(caKeyPath)) {
    logSuccess(`CA certificate already exists at ${caCertPath}`);
    return { caKeyPath, caCertPath };
  }

  try {
    // Generate private key
    execSync(`openssl genrsa -out "${caKeyPath}" 2048`, { stdio: 'pipe' });

    // Generate self-signed certificate
    const subject = '/C=XX/ST=CYNIC/L=Diogenes/O=CYNIC/OU=X-Vision/CN=CYNIC X Vision CA';
    execSync(`openssl req -x509 -new -nodes -key "${caKeyPath}" -sha256 -days 1825 -out "${caCertPath}" -subj "${subject}"`, { stdio: 'pipe' });

    logSuccess(`CA certificate generated at ${caCertPath}`);
    return { caKeyPath, caCertPath };
  } catch (err) {
    // Try with PowerShell on Windows if OpenSSL not available
    if (platform() === 'win32') {
      return generateCACertWindows();
    }
    throw new Error(`Failed to generate CA certificate: ${err.message}`);
  }
}

/**
 * Generate CA cert on Windows using PowerShell (no OpenSSL needed)
 */
function generateCACertWindows() {
  logStep('1/4', 'Generating CA certificate (Windows PowerShell)...');

  if (!existsSync(CERT_DIR)) {
    mkdirSync(CERT_DIR, { recursive: true });
  }

  const caCertPath = join(CERT_DIR, CA_CERT_NAME);
  const caKeyPath = join(CERT_DIR, CA_KEY_NAME);
  const pfxPath = join(CERT_DIR, 'ca.pfx');

  if (existsSync(caCertPath)) {
    logSuccess(`CA certificate already exists at ${caCertPath}`);
    return { caKeyPath, caCertPath };
  }

  const psScript = `
$cert = New-SelfSignedCertificate \`
  -Subject "CN=CYNIC X Vision CA" \`
  -CertStoreLocation "Cert:\\CurrentUser\\My" \`
  -KeyExportPolicy Exportable \`
  -KeyUsage CertSign,CRLSign \`
  -KeyLength 2048 \`
  -HashAlgorithm SHA256 \`
  -NotAfter (Get-Date).AddYears(5) \`
  -TextExtension @("2.5.29.19={text}CA=true")

$pwd = ConvertTo-SecureString -String "cynic" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "${pfxPath.replace(/\\/g, '\\\\')}" -Password $pwd
Export-Certificate -Cert $cert -FilePath "${caCertPath.replace(/\\/g, '\\\\')}" -Type CERT
Write-Output $cert.Thumbprint
`;

  try {
    const result = execSync(`powershell -Command "${psScript.replace(/"/g, '\\"')}"`, { encoding: 'utf8' });
    logSuccess(`CA certificate generated at ${caCertPath}`);
    return { caKeyPath, caCertPath, pfxPath, thumbprint: result.trim() };
  } catch (err) {
    throw new Error(`Failed to generate CA certificate: ${err.message}`);
  }
}

/**
 * Install CA certificate in system trust store
 */
function installCACert(caCertPath) {
  logStep('2/4', 'Installing CA certificate in trust store...');

  const os = platform();

  try {
    if (os === 'win32') {
      // Windows: Use certutil
      execSync(`certutil -addstore -user -f "ROOT" "${caCertPath}"`, { stdio: 'pipe' });
      logSuccess('CA certificate installed in Windows trust store');
    } else if (os === 'darwin') {
      // macOS: Use security command
      execSync(`sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain "${caCertPath}"`, { stdio: 'inherit' });
      logSuccess('CA certificate installed in macOS Keychain');
    } else {
      // Linux: Copy to ca-certificates
      const destPath = '/usr/local/share/ca-certificates/cynic-x-vision.crt';
      execSync(`sudo cp "${caCertPath}" "${destPath}"`, { stdio: 'pipe' });
      execSync('sudo update-ca-certificates', { stdio: 'pipe' });
      logSuccess('CA certificate installed in Linux trust store');
    }
  } catch (err) {
    logWarning(`Could not auto-install certificate: ${err.message}`);
    logWarning('You may need to install manually:');
    log(`  Certificate path: ${caCertPath}`, 'dim');

    if (os === 'win32') {
      log('  Windows: Double-click the .pem file → Install Certificate → Current User → Trusted Root', 'dim');
    } else if (os === 'darwin') {
      log('  macOS: Double-click → Add to Keychain → Always Trust', 'dim');
    } else {
      log('  Linux: Copy to /usr/local/share/ca-certificates/ and run update-ca-certificates', 'dim');
    }
  }
}

/**
 * Configure system proxy settings
 */
function configureProxy(enable = true) {
  logStep('3/4', enable ? 'Configuring system proxy...' : 'Removing proxy configuration...');

  const os = platform();
  const proxyHost = '127.0.0.1';

  try {
    if (os === 'win32') {
      if (enable) {
        // Enable proxy via registry
        execSync(`reg add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings" /v ProxyEnable /t REG_DWORD /d 1 /f`, { stdio: 'pipe' });
        execSync(`reg add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings" /v ProxyServer /t REG_SZ /d "${proxyHost}:${PROXY_PORT}" /f`, { stdio: 'pipe' });
        // Bypass for local addresses
        execSync(`reg add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings" /v ProxyOverride /t REG_SZ /d "localhost;127.*;10.*;192.168.*;<local>" /f`, { stdio: 'pipe' });
        logSuccess(`System proxy configured: ${proxyHost}:${PROXY_PORT}`);
      } else {
        execSync(`reg add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings" /v ProxyEnable /t REG_DWORD /d 0 /f`, { stdio: 'pipe' });
        logSuccess('System proxy disabled');
      }
    } else if (os === 'darwin') {
      const networkService = 'Wi-Fi'; // Could detect this dynamically
      if (enable) {
        execSync(`networksetup -setwebproxy "${networkService}" ${proxyHost} ${PROXY_PORT}`, { stdio: 'pipe' });
        execSync(`networksetup -setsecurewebproxy "${networkService}" ${proxyHost} ${PROXY_PORT}`, { stdio: 'pipe' });
        logSuccess(`System proxy configured: ${proxyHost}:${PROXY_PORT}`);
      } else {
        execSync(`networksetup -setwebproxystate "${networkService}" off`, { stdio: 'pipe' });
        execSync(`networksetup -setsecurewebproxystate "${networkService}" off`, { stdio: 'pipe' });
        logSuccess('System proxy disabled');
      }
    } else {
      // Linux: Set environment variables (user must source or add to profile)
      const proxyUrl = `http://${proxyHost}:${PROXY_PORT}`;
      if (enable) {
        const profilePath = join(homedir(), '.bashrc');
        const proxyConfig = `
# CYNIC X Vision Proxy
export http_proxy="${proxyUrl}"
export https_proxy="${proxyUrl}"
export HTTP_PROXY="${proxyUrl}"
export HTTPS_PROXY="${proxyUrl}"
export no_proxy="localhost,127.0.0.1,::1"
`;
        // Append to .bashrc if not already there
        const existing = existsSync(profilePath) ? readFileSync(profilePath, 'utf8') : '';
        if (!existing.includes('CYNIC X Vision Proxy')) {
          writeFileSync(profilePath, existing + proxyConfig);
          logSuccess(`Proxy config added to ${profilePath}`);
          logWarning('Run: source ~/.bashrc (or restart terminal)');
        } else {
          logSuccess('Proxy config already in .bashrc');
        }
      } else {
        logWarning('Remove CYNIC proxy lines from ~/.bashrc manually');
      }
    }
  } catch (err) {
    logWarning(`Could not auto-configure proxy: ${err.message}`);
    log(`  Configure manually: ${proxyHost}:${PROXY_PORT}`, 'dim');
  }
}

/**
 * Update .env file with X Vision settings
 */
function updateEnvFile() {
  logStep('4/4', 'Updating .env configuration...');

  const envPath = join(__dirname, '..', '.env');
  let envContent = existsSync(envPath) ? readFileSync(envPath, 'utf8') : '';

  const settings = {
    'CYNIC_X_PROXY_ENABLED': 'true',
    'CYNIC_X_PROXY_PORT': PROXY_PORT.toString(),
    'CYNIC_X_PROXY_CERT_PATH': CERT_DIR,
  };

  for (const [key, value] of Object.entries(settings)) {
    const regex = new RegExp(`^${key}=.*$`, 'm');
    const line = `${key}=${value}`;

    if (regex.test(envContent)) {
      envContent = envContent.replace(regex, line);
    } else {
      envContent += `\n${line}`;
    }
  }

  writeFileSync(envPath, envContent.trim() + '\n');
  logSuccess('.env updated with X Vision settings');
}

/**
 * Show status
 */
function showStatus() {
  log('\n🧠 CYNIC X Vision Status\n', 'cyan');

  const caCertPath = join(CERT_DIR, CA_CERT_NAME);

  // Certificate
  if (existsSync(caCertPath)) {
    logSuccess(`CA Certificate: ${caCertPath}`);
  } else {
    logError('CA Certificate: Not generated');
  }

  // Proxy port
  log(`Proxy Port: ${PROXY_PORT}`, 'dim');

  // Check if proxy is running
  try {
    execSync(`curl -s -o /dev/null -w "%{http_code}" --proxy http://127.0.0.1:${PROXY_PORT} http://example.com`, { timeout: 3000 });
    logSuccess('Proxy: Running');
  } catch {
    logWarning('Proxy: Not running (start MCP server with CYNIC_X_PROXY_ENABLED=true)');
  }

  // System proxy config
  const os = platform();
  if (os === 'win32') {
    try {
      const result = execSync('reg query "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings" /v ProxyEnable', { encoding: 'utf8' });
      if (result.includes('0x1')) {
        logSuccess('System Proxy: Enabled');
      } else {
        logWarning('System Proxy: Disabled');
      }
    } catch {
      logWarning('System Proxy: Unknown');
    }
  }

  log('\n');
}

/**
 * Uninstall
 */
function uninstall() {
  log('\n🧠 CYNIC X Vision Uninstall\n', 'cyan');

  // Disable proxy
  configureProxy(false);

  // Remove cert from trust store
  const os = platform();
  const caCertPath = join(CERT_DIR, CA_CERT_NAME);

  try {
    if (os === 'win32') {
      execSync(`certutil -delstore -user "ROOT" "CYNIC X Vision CA"`, { stdio: 'pipe' });
      logSuccess('CA certificate removed from Windows trust store');
    } else if (os === 'darwin') {
      execSync(`sudo security delete-certificate -c "CYNIC X Vision CA"`, { stdio: 'pipe' });
      logSuccess('CA certificate removed from macOS Keychain');
    } else {
      execSync('sudo rm -f /usr/local/share/ca-certificates/cynic-x-vision.crt && sudo update-ca-certificates', { stdio: 'pipe' });
      logSuccess('CA certificate removed from Linux trust store');
    }
  } catch (err) {
    logWarning(`Could not remove certificate: ${err.message}`);
  }

  logSuccess('X Vision uninstalled');
  log('\nNote: Certificate files still in ' + CERT_DIR, 'dim');
  log('Note: .env settings unchanged (set CYNIC_X_PROXY_ENABLED=false)', 'dim');
}

/**
 * Main
 */
async function main() {
  const args = process.argv.slice(2);
  const command = args[0] || '--install';

  log('\n═══════════════════════════════════════════════════════════', 'cyan');
  log('  🧠 CYNIC X Vision Setup', 'cyan');
  log('  "Your data, your device, your choice"', 'dim');
  log('═══════════════════════════════════════════════════════════\n', 'cyan');

  if (command === '--status') {
    showStatus();
    return;
  }

  if (command === '--uninstall') {
    uninstall();
    return;
  }

  // Install flow
  try {
    // 1. Generate CA cert
    const { caCertPath } = generateCACert();

    // 2. Install in trust store
    installCACert(caCertPath);

    // 3. Configure system proxy
    configureProxy(true);

    // 4. Update .env
    updateEnvFile();

    log('\n═══════════════════════════════════════════════════════════', 'green');
    log('  ✓ X Vision Setup Complete!', 'green');
    log('═══════════════════════════════════════════════════════════\n', 'green');

    log('Next steps:', 'cyan');
    log('  1. Restart your browser (to pick up the new CA cert)', 'dim');
    log('  2. Start the MCP server: npm run mcp', 'dim');
    log('  3. Browse x.com - CYNIC will capture your feed', 'dim');
    log('  4. Use brain_x_feed, brain_x_search to query captured data', 'dim');
    log('\nTo disable: node scripts/setup-x-vision.js --uninstall', 'dim');
    log('To check status: node scripts/setup-x-vision.js --status\n', 'dim');

  } catch (err) {
    logError(`Setup failed: ${err.message}`);
    process.exit(1);
  }
}

main();
