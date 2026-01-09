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
const templatePath = path.join(npmDir, 'package.json.tmpl');
const template = fs.readFileSync(templatePath, 'utf8');

const platforms = [
  { name: 'linux-x64', os: 'linux', arch: 'x64', ext: '' },
  { name: 'linux-arm64', os: 'linux', arch: 'arm64', ext: '' },
  { name: 'darwin-x64', os: 'darwin', arch: 'x64', ext: '' },
  { name: 'darwin-arm64', os: 'darwin', arch: 'arm64', ext: '' },
  { name: 'win32-x64', os: 'win32', arch: 'x64', ext: '.exe' },
  { name: 'win32-arm64', os: 'win32', arch: 'arm64', ext: '.exe' },
];

for (const platform of platforms) {
  const pkgName = `coralizer-${platform.os}-${platform.arch}`;
  const pkgDir = path.join(npmDir, pkgName);
  const binDir = path.join(pkgDir, 'bin');
  
  console.log(`Preparing ${pkgName}...`);
  if (fs.existsSync(pkgDir)) {
    fs.rmSync(pkgDir, { recursive: true, force: true });
  }
  fs.mkdirSync(binDir, { recursive: true });
  
  // Copy binary
  const artifactBin = path.join(artifactsDir, `bin-${platform.name}`, `coralizer${platform.ext}`);
  if (!fs.existsSync(artifactBin)) {
    console.error(`Error: Artifact not found at ${artifactBin}`);
    process.exit(1);
  }
  fs.copyFileSync(artifactBin, path.join(binDir, `coralizer${platform.ext}`));
  
  // Generate package.json
  const pkgJson = template
    .split('${PKG_NAME}').join(pkgName)
    .split('${PKG_VERSION}').join(version)
    .split('${PKG_OS}').join(platform.os)
    .split('${PKG_ARCH}').join(platform.arch)
    .split('${PKG_EXT}').join(platform.ext);
  
  fs.writeFileSync(path.join(pkgDir, 'package.json'), pkgJson);
  
  console.log(`Publishing ${pkgName}@${version}...`);
  try {
    execSync('npm publish --access public', { cwd: pkgDir, stdio: 'inherit' });
  } catch (error) {
    console.error(`Failed to publish ${pkgName}:`, error.message);
    process.exit(1);
  }
}

// Publish base package
const basePkgDir = path.join(npmDir, 'app');
const basePkgPath = path.join(basePkgDir, 'package.json');
const basePkg = JSON.parse(fs.readFileSync(basePkgPath, 'utf8'));

basePkg.version = version;
if (basePkg.optionalDependencies) {
  for (const dep in basePkg.optionalDependencies) {
    basePkg.optionalDependencies[dep] = version;
  }
}
fs.writeFileSync(basePkgPath, JSON.stringify(basePkg, null, 2));

console.log(`Publishing base package ${basePkg.name}@${version}...`);
try {
  execSync('npm publish --access public', { cwd: basePkgDir, stdio: 'inherit' });
} catch (error) {
  console.error(`Failed to publish base package:`, error.message);
  process.exit(1);
}
