/**
 * Compare one scenario's records from two runs and name the first checkpoint
 * where they part. The nightly workflow (`engine-fuzz.yml`) gives it the
 * native run's directory and the WebAssembly run's:
 *
 *   node scripts/fuzz-compare.mts <native-dir> <wasm-dir> <id>
 *
 * Exit 0 the runs agree, 1 they diverge, 3 a record is missing or malformed
 * (a harness failure, never read as agreement).
 */
import { compareRuns, describeDivergence, readRecord } from "./fuzz/records.mts";

const [nativeDir, wasmDir, id] = process.argv.slice(2);
if (!nativeDir || !wasmDir || !id) {
  console.error("usage: fuzz-compare <native-dir> <wasm-dir> <id>");
  process.exit(3);
}
const rust = readRecord(nativeDir, id);
const wasm = readRecord(wasmDir, id);
if (!rust || !wasm) {
  console.error(`${id}: missing records in ${rust ? wasmDir : nativeDir}`);
  process.exit(3);
}
try {
  const d = compareRuns(wasm, rust, "WASM");
  if (d) {
    console.log(`${id}: ${describeDivergence(d)}`);
    process.exit(1);
  }
  console.log(`${id}: ${rust.checkpoints.length} checkpoints agree`);
} catch (e) {
  console.error(`${id}: ${(e as Error).message}`);
  process.exit(3);
}
