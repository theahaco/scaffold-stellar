import test from "ava";
import { join } from "path";
import { Binary } from "../src";
import { GithubUrl } from "../src/getBinary";
import { fileExists, inherit, rm } from "../src/utils";

process.env["PATH"] = "";
const isCI = process.env["CI"];
const name = "STELLAR_SCAFFOLD";
const LOCAL_PATH = Binary.DEFAULT_INSTALL_DIR;
const LOCAL_BIN_PATH = join(LOCAL_PATH, name);
const fakeUrl = "https://example.com";
const realUrl = GithubUrl();

const TEST_BIN_DESTINATION = join(__dirname, "..", "test_destination");

test.before(async (t) => {
  await rm(LOCAL_BIN_PATH);
  t.false(await fileExists(LOCAL_BIN_PATH));

  await rm(TEST_BIN_DESTINATION);
  t.false(await fileExists(TEST_BIN_DESTINATION));
});

test("can create", async (t) => {
  const bin = await Binary.create(name, fakeUrl);
  t.is(bin.name, name);
  t.deepEqual(bin.urls[0], new URL(fakeUrl));
  t.is(bin.installDir, LOCAL_PATH);
  t.false(await bin.exists());
});

test("throws if url is bad", async (t) => {
  const bin = await Binary.create(name, fakeUrl);
  const error = await t.throwsAsync(bin.install());
  t.is(error.message, "Failed to download from: \nhttps://example.com/");
});

test("can install and uninstall file", async (t) => {
  const bin = await Binary.create(name, realUrl);
  t.is(bin.name, name);
  t.deepEqual(bin.urls[0], new URL(realUrl));
  t.is(bin.installDir, LOCAL_PATH);
  t.false(await bin.exists());

  t.assert(await bin.install());
  t.assert(await bin.exists());

  await bin.uninstall();
  t.false(await bin.exists());
});

test("can install file to destination", async (t) => {
  const p = TEST_BIN_DESTINATION;
  const bin = await Binary.create(name, realUrl, p);
  await rm(join(p, name));
  t.is(bin.name, name);
  t.deepEqual(bin.urls[0], new URL(realUrl));
  t.not(bin.installDir, LOCAL_PATH);
  t.false(await bin.exists());

  t.assert(await bin.install());
  t.assert(await bin.exists());
});

test("can install file to destination with multiple urls", async (t) => {
  const p = TEST_BIN_DESTINATION;
  const bin = await Binary.create(name, [fakeUrl, realUrl], p);
  await rm(join(p, name));
  t.is(bin.name, name);
  t.deepEqual(bin.urls, [new URL(fakeUrl), new URL(realUrl)]);
  t.not(bin.installDir, LOCAL_PATH);
  t.false(await bin.exists());

  t.assert(await bin.install());
  t.assert(await bin.exists());
});

test("can install and then run", async (t) => {
  const p = join(TEST_BIN_DESTINATION, "install_then_run");
  await rm(join(p, name));
  const bin = await Binary.create(name, realUrl, p);
  await bin.install();
  const stdio = isCI ? [null, inherit, inherit] : [null, null, null];
  const res = await bin.run(["--help"], { stdio });
  t.not(res, 1);
});

test("can run without install", async (t) => {
  const p = join(TEST_BIN_DESTINATION, "to_run");
  await rm(join(p, name));
  const bin = await Binary.create(name, realUrl, p);
  const stdio = isCI ? [null, inherit, inherit] : [null, null, null];
  const res = await bin.run(["--help"], { stdio });
  t.not(res, 1);
});
