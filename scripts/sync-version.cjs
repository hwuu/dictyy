#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

const rootDir = path.join(__dirname, '..');
const versionFile = path.join(rootDir, 'VERSION');

// 读取版本号
const version = fs.readFileSync(versionFile, 'utf8').trim();

if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Invalid version in VERSION file: ${version}`);
  process.exit(1);
}

console.log(`Syncing version: ${version}`);

// 更新 package.json
const pkgPath = path.join(rootDir, 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
if (pkg.version !== version) {
  pkg.version = version;
  fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
  console.log(`  Updated package.json`);
}

// 更新 src-tauri/tauri.conf.json
const tauriConfPath = path.join(rootDir, 'src-tauri', 'tauri.conf.json');
const tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf8'));
if (tauriConf.version !== version) {
  tauriConf.version = version;
  fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
  console.log(`  Updated tauri.conf.json`);
}

// 更新 src-tauri/Cargo.toml
const cargoPath = path.join(rootDir, 'src-tauri', 'Cargo.toml');
let cargoContent = fs.readFileSync(cargoPath, 'utf8');
const cargoVersionMatch = cargoContent.match(/^version = "(.*)"/m);
if (cargoVersionMatch && cargoVersionMatch[1] !== version) {
  cargoContent = cargoContent.replace(/^version = ".*"$/m, `version = "${version}"`);
  fs.writeFileSync(cargoPath, cargoContent);
  console.log(`  Updated Cargo.toml`);
}

console.log('Done');
