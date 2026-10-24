# Grammar oracle

The golden files under `oracle/` are the output of cavemem 0.2.1's caveman
grammar (MIT, Julius Brussee, <https://github.com/JuliusBrussee/cavemem>)
over the texts in `corpus/` (src/grammar T1, V1). The Rust tests in
`src/grammar/tests.rs` compare `grammar::compress` and `grammar::expand`
with them byte for byte; the test suite never runs Node.

## Pin

| what | hash |
| --- | --- |
| `cavemem-0.2.1.tgz` npm integrity | `sha512-sujtPDxbgyFTNT8d3yREFDYLTbkCTTvcBIN0VIvVER/4qtx2cH64b9oEDmL1m0usfpNAo4Il8H058shoGZTFFA==` |
| `cavemem-0.2.1.tgz` sha256 | `de5f577ec12cda0c10e25cb2bf4760155f0220b9ab401ba94e1e704962dae32d` |
| `cavemem-0.2.1.tgz` npm shasum (sha1) | `021af8e7f24d0f69ad7dbce38ff229739878ea3e` |
| `package/dist/chunk-T35V7EPZ.js` sha256 | `03ca2793d362a22902820f4d095988cd976da23b976d494b93dd413ea5029353` |

`oracle/run.mjs` refuses a bundle whose sha256 is not the pinned one.

## Files

For every `corpus/<name>.txt` and every intensity `lite`, `full`, `ultra`:

- `oracle/<name>.<intensity>.compressed`: `compress(text, { intensity })`.
- `oracle/<name>.<intensity>.expanded`: `expand` of that compressed text.

## Regenerate

From the repository root, with the tarball fetched into a new, empty
directory and never installed (no install scripts run):

```sh
dir=target/oracle/cavemem && mkdir -p "$dir"
nix develop --command npm pack cavemem@0.2.1 --pack-destination "$dir"
nix develop --command npm view cavemem@0.2.1 dist.integrity
openssl dgst -sha512 -binary "$dir/cavemem-0.2.1.tgz" | base64
tar -xzf "$dir/cavemem-0.2.1.tgz" -C "$dir"
nix develop --command node tests/grammar/oracle/run.mjs \
  "$dir/package/dist/chunk-T35V7EPZ.js" tests/grammar/corpus tests/grammar/oracle
git diff --exit-code tests/grammar/oracle
```

The two integrity lines must both equal the pin above. The bundle also
imports `better-sqlite3`, so the runner does not import it whole: it
evaluates the grammar's own slice of the bundle, from
`// ../../packages/compress/src/lexicon.json` up to
`// ../../packages/compress/src/privacy.ts`, unchanged.
