# Native QR check-in

`LegacyVenuePackage::check_in(payload)` resolves a scanned payload against the verified package's anchors. It preserves exact matching and ECMAScript whitespace trimming. Only declared QR anchors can resolve; encoding an AprilTag/image/NFC payload in a QR sticker does not make it a QR anchor.

The selected start prefers space/waypoint nodes in the anchor's own space, then standable nodes on its floor, then any node on that floor. Distance ties use UTF-16 ID order independently of serialization order. Unknown codes, unsupported marker kinds and floors without routing nodes return distinct failures. Numeric distance overflow is an error.

The result retains the anchor identity, declared floor/position and selected node. It establishes a map check-in, not proof that a physical sign was placed correctly or that a person remains at that position. A mobile host should show the anchor ID alongside its place label and require a new check-in when location is lost.

The frozen corpus covers every anchor in the four pinned venues, exact payloads, whitespace, suffixes and foreign/empty inputs. Native tests additionally cover non-QR markers, UTF-16 tie-breaking and a floor with no nodes. Record the corpus explicitly with `node --import tsx scripts/record-legacy-checkins.ts --record`.
