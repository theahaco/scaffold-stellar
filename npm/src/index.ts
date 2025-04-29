import { spawn } from "child_process";
import * as fs from "fs/promises";
import got from "got";
import { join } from "path";
import * as stream from "stream";
import * as tar from "tar";
import { URL } from "url";
import { promisify } from "util";
import { fileExists, inherit, rm, searchPath } from "./utils";

const pipeline = promisify(stream.pipeline);

// this is based on https://github.com/raendev/raen/blob/main/npm/src/index.ts

export class Binary {
  urls!: URL[];
  static readonly DEFAULT_INSTALL_DIR = join(__dirname, "..", "bin");

  protected constructor(
    public name: string,
    url: string | URL | string[] | URL[],
    public installDir: string = Binary.DEFAULT_INSTALL_DIR
  ) {
    let errors = [];
    let urls = [];
    if (typeof url === "string" || url instanceof URL) {
      urls.push(url);
    } else {
      if (url.length == 0) {
        throw new Error("No URL provided got empty array");
      }
      urls = url;
    }
    if (!name || typeof name !== "string") {
      errors.push("You must specify the name of your binary as a string");
    }
    try {
      this.urls = urls.map((path) =>
        typeof path === "string" ? new URL(path) : path
      );
    } catch (e) {
      errors.push(e);
    }

    if (errors.length > 0) {
      errors.push(
        '\nCorrect usage: new Binary("my-binary", "https://example.com/binary/download.tar.gz"'
      );
      errors.unshift(
        "One or more of the parameters you passed to the Binary constructor are invalid:\n"
      );
      throw new Error(errors.join("\n"));
    }
  }

  /**
   *
   * @param name binary name, e.g. 'git'
   * @param path URL of where to find binary
   * @param destination Directory to put the binary
   * @returns
   */
  static async create(
    name: string,
    path: string | URL | string[] | URL[],
    destination?: string
  ): Promise<Binary> {
    let installDir =
      destination ?? (await searchPath(name)) ?? Binary.DEFAULT_INSTALL_DIR;
    const bin = new Binary(name, path, installDir);
    await fs.mkdir(installDir!, { recursive: true });
    return bin;
  }

  get binPath(): string {
    return join(this.installDir, this.name);
  }

  download(url: URL): Promise<void> {
    return pipeline(
      got.stream(url),
      new stream.PassThrough(),
      tar.x({ C: this.installDir })
    );
  }

  async install(): Promise<boolean> {
    for (let url of this.urls) {
      try {
        await this.download(url);
        return true;
      } catch (error: unknown) {}
    }
    throw new Error(`Failed to download from: \n${this.urls.join("\n")}`);
  }

  async exists(): Promise<boolean> {
    return await fileExists(this.binPath);
  }

  async run(
    cliArgs?: string[],
    options = { stdio: [null, inherit, inherit] }
  ): Promise<number> {
    if (!(await this.exists())) {
      try {
        await this.install();
      } catch (err) {
        console.error(err);
        return 1;
      }
    }

    const args = cliArgs ?? process.argv.slice(2);
    const result = spawn(this.binPath, args, options);
    result.on("error", (error) => {
      console.log(error);
    });

    return new Promise((resolve, reject) => {
      result.on("close", (code) => {
        if (!code) {
          resolve(code ?? 0);
        }
        reject(code);
      });
    });
  }

  async runAndExit(
    cliArgs?: string[],
    options = { stdio: [null, inherit, inherit] }
  ): Promise<void> {
    process.exit(await this.run(cliArgs, options));
  }

  async uninstall(): Promise<void> {
    if (
      this.installDir === Binary.DEFAULT_INSTALL_DIR &&
      (await this.exists())
    ) {
      await rm(this.binPath);
      if (await this.exists()) {
        throw new Error(`Failed to remove binary located ${this.binPath}`);
      }
    }
  }
}
