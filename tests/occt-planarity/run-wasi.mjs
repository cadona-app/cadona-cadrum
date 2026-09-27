import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";

if (process.argv.length !== 3) {
  throw new Error("Pass the planarity WebAssembly executable path");
}

const wasi = new WASI({
  version: "preview1",
  args: [],
  env: {},
  preopens: {},
  returnOnExit: true,
});
const module = await WebAssembly.compile(await readFile(process.argv[2]));
const instance = await WebAssembly.instantiate(module, {
  wasi_snapshot_preview1: wasi.wasiImport,
});
process.exitCode = wasi.start(instance);
