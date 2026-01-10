const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const version = process.env.VERSION;
if (!version) {
  console.error('Error: VERSION environment variable is not set.');
  process.exit(1);
}

const artifactsDir = path.join(process.cwd(), 'artifacts');
const npmDir = path.join(process.cwd(), 'npm');
const appDir = path.join(npmDir, 'app');
const binDir = path.join(appDir, 'bin');

console.log('Preparing binaries...');
if (fs.existsSync(binDir)) {
  fs.rmSync(binDir, { recursive: true, force: true });
}
fs.mkdirSync(binDir, { recursive: true });

const platforms = [
  { name: 'linux-x64', os: 'linux', arch: 'x64', ext: '' },
  { name: 'linux-arm64', os: 'linux', arch: 'arm64', ext: '' },
  { name: 'darwin-x64', os: 'darwin', arch: 'x64', ext: '' },
  { name: 'darwin-arm64', os: 'darwin', arch: 'arm64', ext: '' },
  { name: 'win32-x64', os: 'win32', arch: 'x64', ext: '.exe' },
  { name: 'win32-arm64', os: 'win32', arch: 'arm64', ext: '.exe' },
];

for (const platform of platforms) {
  const artifactBin = path.join(artifactsDir, `bin-${platform.name}`, `coralizer${platform.ext}`);
  if (!fs.existsSync(artifactBin)) {
    console.error(`Error: Artifact not found at ${artifactBin}`);
    process.exit(1);
  }
  
  const destName = `coralizer-${platform.os}-${platform.arch}${platform.ext}`;
  const destPath = path.join(binDir, destName);
  
  console.log(`Copying ${artifactBin} to ${destPath}...`);
  fs.copyFileSync(artifactBin, destPath);
  
  // Ensure executable permissions on Unix
  if (platform.os !== 'win32') {
    fs.chmodSync(destPath, 0o755);
  }
}

// Update package.json version
const pkgPath = path.join(appDir, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = version;
fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2));

console.log(`Publishing ${pkg.name}@${version}...`);
try {
  execSync('npm publish --access public', { cwd: appDir, stdio: 'inherit' });
} catch (error) {
  console.error(`Failed to publish:`, error.message);
  process.exit(1);
}
