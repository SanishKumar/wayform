# Native legacy venue packages

`legacy_package_content_hash` reproduces the pinned VoiceGIS package content identity: exclude the root manifest, sort object keys, serialize with two-space indentation and ECMAScript number formatting, append a newline, then calculate SHA-256. Nested fields named `manifest` remain part of the content. This format is separate from the mobility draft's compact identity format.

The native serializer accepts ASCII identifier property names (a letter followed by letters or digits). All keys in the four pinned packages meet this boundary. It reproduces their reference runtime's case-insensitive primary ordering and lowercase-before-uppercase ties. Arbitrary Unicode, punctuation and numeric property names are rejected rather than approximating locale-dependent collation. String values can contain Unicode. The maximum depth is 64 and the serialized content is limited to 16 MiB.

Four package hashes and additional frozen number, Unicode-value, case-ordering and nested-manifest examples are checked against the unchanged TypeScript implementation. Normal tests never rewrite expectations. Record the additional cases explicitly with `node --import tsx scripts/record-legacy-package-identities.ts --record`.

This function only computes content identity. It does not validate package shape, verify the claimed manifest, authenticate a publisher or establish survey quality. Full native loading is the next integration step; existing routing still receives host-verified projections.
