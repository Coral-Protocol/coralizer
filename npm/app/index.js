#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const fs = require('fs');

function getExePath() {
  const arch = process.arch;
  const platform = process.platform;
  const extension = platform === "win32" ? ".exe" : "";

  const exeName = `coralizer-${platform}-${arch}${extension}`;
  const exePath = path.join(__dirname, 'bin', exeName);

  if (fs.existsSync(exePath)) {
    return exePath;
  } else {
    console.error(`Error: Could not find the binary for your platform (${platform}-${arch}) at ${exePath}.`);
    process.exit(1);
  }
}

function run() {
  const exePath = getExePath();
  const args = process.argv.slice(2);
  const result = spawnSync(exePath, args, { stdio: 'inherit' });
  
  if (result.error) {
    console.error('Error executing binary:', result.error);
    process.exit(1);
  }
  
  process.exit(result.status ?? 0);
}

run();
