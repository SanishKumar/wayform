# Native legacy directions

`LegacyVenuePackage::route_directions` calculates a snapshot legacy path and derives its base steps from verified runtime metadata. It preserves corridor and room wording, turn thresholds, zero-length joins, doorway hops, consecutive lift/stair legs, floor changes and arrival instructions from the pinned `routingCore.ts`. A missing or unreachable endpoint returns no directions. Numerical overflow is rejected.

This is an intermediate instruction component. Landmark wording, graphic clearance and the legacy route receipt are not included yet. It is not a live navigation session: callers must resolve closures and gate every displayed/spoken instruction on current policy and location before using guidance. Authored accessibility flags remain unmeasured assertions.

All 7,238 pinned journeys are rerun through the original base generator and the native component. Four frozen aggregate digests compare complete path metadata and steps in the original journey order. Only bearings are rounded to nine decimal places for these digests, accounting for platform `atan2` rounding; text, distance, IDs and other metadata are exact. The original full-route fingerprints remain untouched. Stationary, unknown and fully closed routes are tested separately.

Record base-direction expectations explicitly with `node --import tsx scripts/record-legacy-directions.ts --record`. Normal tests never update them. The original TypeScript implementation remains the parity reference until the complete route result is native.
