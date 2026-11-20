/**
 * Compare one scenario's records from two runs and name the first checkpoint
 * where they part. The nightly workflow (`engine-fuzz.yml`) gives it the
 * native run's directory and the WebAssembly run's:
 *
 *   node scripts/fuzz-compare.mts <native-dir> <wasm-dir> <id>
 *
 * Exit 0 the runs agree, 6 they diverge, 3 a record is missing or malformed
 * (a harness failure, never read as agreement). Node exits 1 when the script
 * itself crashes, so no finding uses 1.
 */
const DIVERGED = 6;
const HARNESS = 3;

try {
  const { compareRuns, describeDivergence, readRecord } = await import("./fuzz/records.mts");
  const [nativeDir, wasmDir, id] = process.argv.slice(2);
  if (!nativeDir || !wasmDir || !id) {
    console.error("usage: fuzz-compare <native-dir> <wasm-dir> <id>");
    process.exit(HARNESS);
  }
  const rust = readRecord(nativeDir, id);
  const wasm = readRecord(wasmDir, id);
  if (!rust || !wasm) {
    console.error(`${id}: missing records in ${rust ? wasmDir : nativeDir}`);
    process.exit(HARNESS);
  }
  const d = compareRuns(wasm, rust, "WASM");
  if (d) {
    console.log(`${id}: ${describeDivergence(d)}`);
    process.exit(DIVERGED);
  }
  console.log(`${id}: ${rust.checkpoints.length} checkpoints agree`);
} catch (e) {
  console.error(`fuzz-compare: ${(e as Error).message}`);
  process.exit(HARNESS);
}
