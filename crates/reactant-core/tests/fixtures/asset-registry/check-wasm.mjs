import { readFile } from "node:fs/promises";

const bytes = await readFile(process.argv[2]);
const imports = {
  __wbindgen_placeholder__: {
    __wbindgen_describe() {},
    __wbg___wbindgen_throw_bb96b2010945f0bc() {
      throw new Error("unexpected wasm-bindgen throw");
    },
  },
  __wbindgen_externref_xform__: {
    __wbindgen_externref_table_grow() {
      return 0;
    },
    __wbindgen_externref_table_set_null() {},
  },
};
const { instance } = await WebAssembly.instantiate(bytes, imports);
const count = instance.exports.registration_count();
const hash = instance.exports.registration_address_hash();

if (count !== 2 || hash === 0) {
  throw new Error(`unexpected registry: count=${count} hash=${hash}`);
}

console.log(`registrations=${count} address_hash=${hash.toString(16).padStart(8, "0")}`);
