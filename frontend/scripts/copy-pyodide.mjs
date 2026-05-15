import { cp, mkdir } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const source = resolve(root, 'node_modules', 'pyodide');
const target = resolve(root, 'public', 'pyodide');

await mkdir(target, { recursive: true });
await cp(source, target, {
  recursive: true,
  force: true,
  filter: (path) => !path.includes('/node_modules/'),
});
