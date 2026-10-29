import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { test, beforeEach, afterEach } from 'node:test';
import { rspack } from '@rspack/core';

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'translation-converter-'));
const entry = path.join(dir, 'entry.js');

beforeEach(() => {
  fs.writeFileSync(entry, `
    const t = value => value;
    const other = { common: { foo: 'untouched' } };
    const key = 'bar';
    export default [t(l.common.fooBar), t(l.common.foo[key]), t(other.common.foo)];
  `);
});

afterEach(() => {
  fs.rmSync(dir, { recursive: true, force: true });
});

test('Rspack runs the WASM plugin through builtin:swc-loader', async () => {
  const compiler = rspack({
    mode: 'development',
    target: 'node',
    devtool: false,
    entry,
    output: { path: dir, filename: 'bundle.cjs', library: { type: 'commonjs2' } },
    module: {
      rules: [{
        test: /\.js$/,
        use: {
          loader: 'builtin:swc-loader',
          options: {
            jsc: {
              experimental: {
                cacheRoot: path.join(dir, 'swc-cache'),
                plugins: [[path.resolve(import.meta.dirname, '../swc_plugin_translation_converter.wasm'), {}]],
              },
            },
          },
        },
      }],
    },
  });

  const stats = await new Promise((resolve, reject) => {
    compiler.run((error, result) => {
      compiler.close(closeError => {
        if (error || closeError) {
          return reject(error || closeError);
        }

        resolve(result);
      });
    });
  });

  assert.equal(stats.hasErrors(), false, stats.toString({ all: false, errors: true, errorDetails: true }));

  const bundle = await import(pathToFileURL(path.join(dir, 'bundle.cjs')).href);
  assert.deepEqual(bundle.default.default, [
    'common:fooBar', 'common:foo.bar', 'untouched',
  ]);
});
