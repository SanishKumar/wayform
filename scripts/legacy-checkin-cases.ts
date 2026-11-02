import { loadVenue, venues } from '../compat/voicegis/harness';
import { checkInFromScan } from '../compat/voicegis/upstream/src/capture/anchorCheckIn';

export async function legacyCheckinCases() {
  return (await Promise.all(venues.map(async venue => {
    const pkg = await loadVenue(venue);
    const payloads = ['', ' ', 'foreign:venue:anchor', ...pkg.localizationAnchors.flatMap(anchor => [anchor.payload, `\uFEFF\t${anchor.payload}\u3000`, `${anchor.payload}/suffix`, `\u0085${anchor.payload}`])];
    return payloads.map((payload,index) => ({ id:`${venue}/${index}`,venue,payload,expected:checkInFromScan(payload,pkg.localizationAnchors,pkg.routing.nodes) }));
  }))).flat();
}
