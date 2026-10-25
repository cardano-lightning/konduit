import { DatabaseSync } from "node:sqlite";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const databasePath =
  process.env.KONDUIT_INDEXER_DB_PATH ??
  join(repoRoot, ".run", "konduit-indexer", "konduit-indexer.db");
const db = new DatabaseSync(databasePath, {
  readOnly: true,
  enableForeignKeyConstraints: false,
  timeout: 5000,
});
let last = -1;
for (let i = 0; i < 600; i++) {
  const s = Date.now();
  let row;
  try {
    row = db
      .prepare(
        "SELECT block_no, header_hash, slot_no FROM block ORDER BY block_no DESC LIMIT 1",
      )
      .get();
  } catch (e) {
    console.log(i, "QUERY THREW after", Date.now() - s, "ms:", String(e));
    break;
  }
  const took = Date.now() - s;
  if (took > 100) console.log(i, "SLOW query", took, "ms");
  if (row?.slot_no !== last) {
    last = row?.slot_no;
    console.log(i, "slot", last, "took", took, "ms", new Date().toISOString());
  }
  await new Promise((r) => setTimeout(r, 500));
}
console.log("watcher finished");
