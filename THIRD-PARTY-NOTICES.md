# Third-party notices

chertmem includes work from the projects below, and the `chert` binary
links two C libraries compiled into it. Each notice is reproduced as its
license requires.

## cavemem 0.2.1

chert is a rewrite of cavemem 0.2.1,
<https://github.com/JuliusBrussee/cavemem>, published on npm as
`cavemem@0.2.1`, under the MIT license below. From it chert takes:

- **The caveman grammar.** `src/grammar.rs` and `src/grammar/` are
  ported from cavemem's `packages/compress` (`lexicon.json`,
  `tokenize.ts`, `compress.ts`, `expand.ts`), and the grammar tests
  compare chert's output with cavemem's own JavaScript, byte for byte,
  except where `src/grammar/SPEC.md` records a deliberate deviation.
- **The hook and MCP shapes.** The set of hook events and the way they
  are wired into Claude Code (one hook group per event calling
  `<binary> hook run <event>`), the session and turn summaries, and the
  four MCP tools (`search`, `timeline`, `get_observations`,
  `list_sessions`) with their parameter and field names. `chert import
  --from cavemem` reads cavemem's database schema.

The license below is the `LICENSE` file of the `cavemem@0.2.1` npm
tarball, unchanged.

```text
MIT License

Copyright (c) 2026 Julius Brussee

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## SQLite

The store is SQLite, with FTS5, compiled into `chert` from the
amalgamation that the `libsqlite3-sys` crate bundles (the `rusqlite`
`bundled` feature); `Cargo.lock` pins the version. SQLite is in the
public domain: <https://sqlite.org/copyright.html>. Its source carries,
in place of a legal notice, this blessing:

```text
The author disclaims copyright to this source code.  In place of
a legal notice, here is a blessing:

   May you do good and not evil.
   May you find forgiveness for yourself and forgive others.
   May you share freely, never taking more than you give.
```

## Zstandard

Cold partitions are packed with the Zstandard C library, compiled into
`chert` from the copy the `zstd-sys` crate bundles; `Cargo.lock` pins
the version. It is used under the BSD license below, the `LICENSE`
file of that copy, unchanged (Zstandard is also offered under
GPL-2.0; chert takes the BSD option).

```text
BSD License

For Zstandard software

Copyright (c) Meta Platforms, Inc. and affiliates. All rights reserved.

Redistribution and use in source and binary forms, with or without modification,
are permitted provided that the following conditions are met:

 * Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

 * Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

 * Neither the name Facebook, nor Meta, nor the names of its contributors may
   be used to endorse or promote products derived from this software without
   specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## Rust crates

Every other dependency is a Rust crate under MIT, or MIT OR Apache-2.0,
or another license `deny.toml` allows; `cargo deny check licenses` is a
gate step, and each direct dependency's reason and license is noted
beside it in [`Cargo.toml`](Cargo.toml). Their notices travel with their
sources on crates.io.
