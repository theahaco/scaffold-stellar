"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getBinary = exports.GithubUrl = void 0;
const os = require("os");
const path_1 = require("path");
const _1 = require(".");
const { version } = require("../package.json");
const NAME = "loam";
function getPlatform() {
    const type = os.type();
    const arch = os.arch();
    let typeDict = {
        Darwin: "apple-darwin",
        Linux: "unknown-linux-gnu",
        Windows_NT: "pc-windows-msvc",
    };
    let archDict = {
        x64: "x86_64",
        arm64: "aarch64",
    };
    //@ts-ignore
    let rust_type = typeDict[type];
    //@ts-ignore
    let rust_arch = archDict[arch];
    if (rust_type && rust_arch) {
        return [rust_type, rust_arch];
    }
    throw new Error(`Unsupported platform: ${type} ${arch}`);
}
function GithubUrl() {
    const [platform, arch] = getPlatform();
    return `https://github.com/loambuild/loam-sdk/releases/download/loam-cli-v${version}/loam-cli-v${version}-${arch}-${platform}.tar.gz`;
}
exports.GithubUrl = GithubUrl;
function getBinary(name = NAME) {
    if (!process.env["LOAM_BIN_PATH"]) {
        process.env["LOAM_BINARY_PATH"] = (0, path_1.join)(os.homedir(), `.${NAME}`, NAME);
    }
    // Will use version after publishing to AWS
    // const version = require("./package.json").version;
    const fromEnv = process.env["LOAM_ARTIFACT_URL"];
    const urls = [GithubUrl()];
    if (fromEnv) {
        urls.unshift(fromEnv);
    }
    return _1.Binary.create(name, urls);
}
exports.getBinary = getBinary;
