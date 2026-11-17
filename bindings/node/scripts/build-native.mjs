// Builds the Rust cdylib and copies it to native/virgostars_ephemeris.node.
// Needs a Rust toolchain; downloads nothing except Cargo crates already pinned
// in Cargo.lock. It never fetches ephemeris or catalog data.
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const result = spawnSync('cargo', ['build', '--release', '--locked'], { cwd: root, stdio: 'inherit', shell: process.platform === 'win32' });
if (result.status !== 0) process.exit(result.status ?? 1);
const lib = { win32: 'virgostars_ephemeris_node.dll', darwin: 'libvirgostars_ephemeris_node.dylib', linux: 'libvirgostars_ephemeris_node.so' }[process.platform];
if (!lib) throw new Error(`unsupported platform ${process.platform}`);
mkdirSync(join(root, 'native'), { recursive: true });
copyFileSync(join(root, 'target', 'release', lib), join(root, 'native', 'virgostars_ephemeris.node'));
console.log('built native/virgostars_ephemeris.node');
