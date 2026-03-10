#!/usr/bin/env node

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { createGunzip } = require("zlib");

const REPO = "clichedmoog/gtm-cli";
const BIN_NAME = process.platform === "win32" ? "gtm.exe" : "gtm";
const BIN_DIR = path.join(__dirname, "bin");

const PLATFORM_MAP = {
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "darwin-x64": "x86_64-apple-darwin",
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function getTarget() {
  const key = `${process.platform}-${process.arch}`;
  const target = PLATFORM_MAP[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`);
    process.exit(1);
  }
  return target;
}

function getVersion() {
  const pkg = require("./package.json");
  return pkg.version;
}

function fetch(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "gtm-cli-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          return fetch(res.headers.location).then(resolve, reject);
        }
        if (res.statusCode !== 200) {
          return reject(new Error(`HTTP ${res.statusCode}: ${url}`));
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function install() {
  const target = getTarget();
  const version = getVersion();
  const isWindows = process.platform === "win32";
  const ext = isWindows ? "zip" : "tar.gz";
  const assetName = `gtm-${target}.${ext}`;
  const url = `https://github.com/${REPO}/releases/download/v${version}/${assetName}`;

  console.log(`Downloading gtm v${version} for ${target}...`);

  try {
    const data = await fetch(url);

    fs.mkdirSync(BIN_DIR, { recursive: true });

    if (isWindows) {
      // Write zip and extract with PowerShell
      const zipPath = path.join(BIN_DIR, assetName);
      fs.writeFileSync(zipPath, data);
      execSync(
        `powershell -Command "Expand-Archive -Force '${zipPath}' '${BIN_DIR}'"`,
        { stdio: "inherit" }
      );
      fs.unlinkSync(zipPath);
    } else {
      // Extract tar.gz
      const tarPath = path.join(BIN_DIR, assetName);
      fs.writeFileSync(tarPath, data);
      execSync(`tar xzf "${tarPath}" -C "${BIN_DIR}"`, { stdio: "inherit" });
      fs.unlinkSync(tarPath);
      fs.chmodSync(path.join(BIN_DIR, BIN_NAME), 0o755);
    }

    console.log(`Installed gtm to ${path.join(BIN_DIR, BIN_NAME)}`);
  } catch (err) {
    console.error(`Failed to download gtm: ${err.message}`);
    console.error(`URL: ${url}`);
    console.error(
      "\nYou can install manually from: " +
        `https://github.com/${REPO}/releases`
    );
    process.exit(1);
  }
}

install();
