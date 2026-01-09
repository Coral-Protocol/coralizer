#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');

function getExePath() {
  const arch = process.arch;
  const platform = process.platform;
  const extension = platform === "win32" ? ".exe" : "";

  const pkgName = `coralizer-${platform}-${arch}`;
  try {
    return require.resolve(pkgName);
  } catch (e) {
    console.error(`Error: Could not find the binary for your platform (${platform}-${arch}).`);
    console.error(`Please ensure that the optional dependency '${pkgName}' is installed.`);
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
