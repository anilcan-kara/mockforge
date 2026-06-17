#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { spawn } = require('child_process');

const VERSION = '0.1.2';
const REPO = 'anilcan-kara/mockforge';

// Map Node platform/arch to release assets
const platform = process.platform;
const arch = process.arch;

let osKey = '';
let ext = '';

if (platform === 'win32') {
  osKey = 'win32';
  ext = '.exe';
} else if (platform === 'darwin') {
  osKey = 'darwin';
} else if (platform === 'linux') {
  osKey = 'linux';
} else {
  console.error(`[MockForge] Unsupported platform: ${platform}`);
  process.exit(1);
}

let archKey = '';
if (arch === 'x64') {
  archKey = 'x64';
} else if (arch === 'arm64') {
  archKey = 'arm64';
} else {
  console.error(`[MockForge] Unsupported architecture: ${arch}`);
  process.exit(1);
}

const binaryName = `mockforge-${osKey}-${archKey}${ext}`;
const targetDir = path.join(__dirname, '..', 'dist');
const binaryPath = path.join(targetDir, `mockforge${ext}`);
const downloadUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/${binaryName}`;

function downloadFile(url, dest, callback) {
  https.get(url, (res) => {
    if (res.statusCode === 302 || res.statusCode === 301) {
      downloadFile(res.headers.location, dest, callback);
      return;
    }
    if (res.statusCode !== 200) {
      callback(new Error(`Failed to download binary: HTTP ${res.statusCode}`));
      return;
    }
    const file = fs.createWriteStream(dest);
    res.pipe(file);
    file.on('finish', () => {
      file.close(callback);
    });
  }).on('error', (err) => {
    fs.unlink(dest, () => {});
    callback(err);
  });
}

function runBinary() {
  const args = process.argv.slice(2);
  const child = spawn(binaryPath, args, { stdio: 'inherit' });

  child.on('close', (code) => {
    process.exit(code === null ? 1 : code);
  });

  child.on('error', (err) => {
    console.error(`[MockForge] Failed to start gateway binary:`, err);
    process.exit(1);
  });
}

// Check if binary exists
if (fs.existsSync(binaryPath)) {
  runBinary();
} else {
  console.log(`\x1b[36m[MockForge]\x1b[0m Downloading platform binary v${VERSION} (${osKey}-${archKey})...`);
  
  if (!fs.existsSync(targetDir)) {
    fs.mkdirSync(targetDir, { recursive: true });
  }

  downloadFile(downloadUrl, binaryPath, (err) => {
    if (err) {
      console.error(`\x1b[31m[MockForge] Download failed:\x1b[0m`, err.message);
      console.error(`Please download MockForge manually from https://github.com/${REPO}/releases`);
      process.exit(1);
    }

    // Set execute permissions
    if (platform !== 'win32') {
      try {
        fs.chmodSync(binaryPath, 0755);
      } catch (chmodErr) {
        console.warn(`[MockForge] Failed to set permissions on binary:`, chmodErr);
      }
    }

    console.log(`\x1b[32m[MockForge] Binary downloaded successfully!\x1b[0m\n`);
    runBinary();
  });
}
