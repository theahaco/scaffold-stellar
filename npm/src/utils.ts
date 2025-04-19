import { rm as RM, stat } from "fs/promises";
import { join } from "path";

export async function fileExists(s: string): Promise<boolean> {
  try {
    const f = await stat(s);
    return f.isFile();
  } catch {
    return false;
  }
}

export async function searchPath(
  filename: string
): Promise<string | undefined> {
  const binPath = process.env["LOAM_BINARY_PATH"];
  if (
    binPath &&
    binPath.length > 0 &&
    (await fileExists(join(binPath, filename)))
  ) {
    return binPath;
  }

  return undefined;
}

export const inherit: "inherit" = "inherit";

export async function rm(path: string): Promise<void> {
  try {
    await RM(path);
  } catch (e) {}
}
