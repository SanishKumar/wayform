# Native route graphic assessment

`LegacyVenuePackage::route_display_clearance` evaluates the selected legacy path against its authored spaces, portals and connector stops. It preserves the pinned guard's containing-space checks, policy exclusions, continuous segment containment, shared-wall openings and swept 0.7-metre graphic envelope. Used spaces retain traversal order; only portals crossed by the path remove walls. A stationary route still requires a containing space and room for its graphic.

The result distinguishes an invalid centerline, which cannot guide, from a valid path whose graphic is withheld. It records ordered, deduplicated issues with the package hash and selected profile. An absent path produces no assessment. This standalone component does not itself publish a complete route or retain a policy lease.

This is a rendering check on authored geometry, not a person's width requirement or a survey certificate. It does not account for furniture or live obstacles. The native port currently exposes the assessment; it does not expose the original renderer's arbitrary disc/polygon fit callbacks or AR chevron footprint helper. The mobile renderer must keep any route stroke within the assessed envelope and use separate screen-space destination markers.

Geometry work is conservatively bounded before intersection checks, and extreme coordinate magnitudes are rejected to avoid overflowing products. Existing package limits and search restrictions still apply. The pinned compiler and guard use different small-segment thresholds in their distance predicates; both are preserved.

Four frozen hashes cover all 7,238 original graphic assessments. Eleven rehashed negative packages check room/portal/connector policy exclusions, shifted portals/stops, an outside room, a narrow doorway and an opening that extends beyond its wall. The tests compare complete assessments and issue order against the unchanged TypeScript guard. Record expectations explicitly with `node --import tsx scripts/record-legacy-clearance.ts --record`.
