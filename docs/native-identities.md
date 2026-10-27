# Native draft input identities

`bundle_digest` and `input_digests` reproduce the input identities used by mobility contract `0.1.0-draft.1`. They normalize the declared unordered collections, serialize compact UTF-8 JSON using ECMAScript-compatible binary64 number formatting, and compute SHA-256. They do not perform full schema validation or authenticate a publisher.

Object keys follow the existing TypeScript implementation: array-index property names from `0` through `4294967294` appear in numeric order, followed by other keys in UTF-16 lexical order. This reflects `JSON.stringify` enumeration after `Object.fromEntries`; it is not a generic JCS encoding. Negative zero serializes as zero. Large integer JSON tokens are interpreted as binary64 values when hashing, matching JavaScript's numeric representation. Revisions and step counts still require separate safe-integer validation.

Bundle collections are sorted by ID, control pairs by their canonical JSON strings, and set-valued lists by UTF-16 order. Ordered coordinate arrays retain their order. Profile and context normalization follows the TypeScript contract, including policy requirement IDs and closed-edge lists.

`canonical_json` limits output to 16 MiB and nesting to 64 levels. Native strings contain Unicode scalar values; inputs containing lone UTF-16 surrogates cannot be represented. A native loader must reject those inputs rather than replace characters and compute a different identity.

## Checks

Native tests compare all ten original portable receipt identities plus 16 additional numeric, escaping, Unicode and property-order cases. Collection permutations must preserve the resulting digests. `npm run check` verifies the committed additional cases against TypeScript; `cargo test --workspace --locked` verifies the native implementation. Record expectations only with `node --import tsx scripts/record-identity-examples.ts --record` and review changed digests.

The hashing code uses RustCrypto SHA-256 and `ryu-js` number formatting. The draft identity format remains versioned; changing serialization or collection normalization requires reviewing stored receipts and policy bindings.
