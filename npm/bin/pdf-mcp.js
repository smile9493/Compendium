#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

const platform = os.platform();
const arch = os.arch();

let binaryName;
if (platform === 'win32') {
  binaryName = 'pdf-mcp.exe';
} else if (platform === 'darwin') {
  binaryName = arch === 'arm64' ? 'pdf-mcp-macos-arm64' : 'pdf-mcp-macos-x64';
} else {
  binaryName = 'pdf-mcp-linux-x64';
}

const binaryPath = path.join(__dirname, '..', 'binaries', binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`Binary not found for platform: ${platform}-${arch}`);
  console.error(`Expected path: ${binaryPath}`);
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: {
    ...process.env,
    PDFIUM_LIB_PATH: process.env.PDFIUM_LIB_PATH || path.join(__dirname, '..', 'binaries', 'libpdfium.so')
  }
});

child.on('exit', (code) => {
  process.exit(code || 0);
});
