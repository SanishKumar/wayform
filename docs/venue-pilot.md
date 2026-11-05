# Prepare one venue for a supervised pilot

Pick one journey a visitor actually needs, such as the public drop-off point to a reception desk. Include a second entrance or corridor if a visitor might need a wider path. Keep the first survey small enough for the venue to recheck when a door, lift or route changes.

## What to request from the venue

| Item | What to collect |
| --- | --- |
| Permission and ownership | Written permission to use the plan and test the route, the parts that may be published, and a contact who can correct the map or withdraw a route. Keep private plans out of the public repository. |
| Plan and endpoints | A dated plan with scale, units, floor labels, public/restricted boundaries, entrance and destination IDs, and names matching the signs on site. |
| Outdoor connection | The entrance coordinates, coordinate reference and accuracy, paired with control points on the local plan. Record how the frames were aligned and check the residual error against independent points. |
| Directed connections | Where each segment begins and ends, permitted travel directions, length, door or lift operation, and any hours or closure dependencies. Check the reverse direction separately. |
| Measurements | Narrowest clear passage width in metres, longitudinal slope in percent along each direction, cross-slope where relevant, step count and surface. Record the instrument or method and its uncertainty. Leave unknown values explicit. |
| Evidence and changes | Observation time in UTC, surveyor ID, review status, reviewer and review time, expiry or reassessment date, and which edge each observation covers. Agree who reports closures and how quickly staff can withdraw guidance. |

## Review a file on Android

1. Use a copy of the [fictional example](../examples/survey/clinic-entrance.json) to learn the shape, then author a separate bundle for the site. Replace the topology, measurements, evidence and identities with the venue's records. The [contract](mobility-contract.md) defines units, time syntax, uncertainty, frames and directed observations.
2. Transfer the JSON file privately to the phone. Open **Survey routes → Open survey file**, select the two endpoints, enter the visitor's requirements and tap **Check route**. The import limit is 1 MiB. File contents stay in memory while the review screen is open.
3. Compare the selected segments with the survey notes. Try a requirement known to exclude one passage, an unmeasured attribute and the reverse direction. Check that the observed outcome matches the records. Record the bundle revision, endpoints, requirements, evaluation time and evidence IDs with each observation.
4. The app checks imported observations against the device clock. An expired example imported through Files should report missing current evidence. The in-app **Try the clinic example** option deliberately replays its recorded time.

The Android review accepts reviewed field-survey records from the selected file. Publisher identities and live operational updates still need an authenticated source. Edges covered by policy requirements remain unavailable in review if the policy is missing. The sample venues provide a separate manual guidance flow for testing QR signs, speech and recovery.

## Field trial and release gates

- Walk both the preferred route and its alternative with the venue operator and intended users under supervision. Use the venue's existing directions as a fallback. Record missed turns, barriers, sign visibility, location-recovery attempts and staff interventions without collecting personal medical details.
- On physical phones, check QR recognition in the actual lighting, offline speech audibility, large text, screen-reader navigation, and suspension and recovery at a real decision point. Log the device, OS and bundle revision.
- Before offering live guidance for a surveyed venue, bind the reviewed graph to the installed map and signs, authenticate its publisher and current closure revisions, and withdraw guidance on expiry or location loss. Repeat the walk after corrections and have the venue approve the specific route and its maintenance owner.
