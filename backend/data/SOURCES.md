# Geographic and road model provenance

Boundary download (checked-in bytes):
https://raw.githubusercontent.com/longwosion/geojson-map-china/0999e0343308443cf9a2a1490c3eaeea576eb181/geometryProvince/32.json

Repository: https://github.com/longwosion/geojson-map-china

Retrieved 2026-11-16. SHA-256: `852d8bdad9b4bfb0d1d3dbd54cdd5c2bb22268bb8380a8a66aa026287e25a7c5`.
13 city features, historic generalized boundaries, source `cp` centers. Source
IDs are four digits; backend normalizes them to six-digit administrative IDs.
The source does not document a datum or an explicit data license. Do not claim
survey accuracy, current official boundaries, or an open-data license. Keep
these coordinates in their native frame for the boundary-only visualization;
no external WGS84 basemap overlay. Spherical distances interpret the native
longitude/latitude numerically and are approximate model costs, not measured
road distances. A documented WGS84/GCJ02 source can replace this dataset later.

Initially proposed source (not downloaded; current proxy rejects destination):
https://geo.datav.aliyun.com/areas_v3/bound/320000_full.json

`roads.json` is the explicitly proposed 23-edge manual city-level abstraction,
not extracted road geometry. Lines connect representative centers. Cross-river
edges represent highway bridges (Nanjing–Yangzhou, Zhenjiang–Yangzhou,
Zhenjiang–Taizhou, Wuxi–Taizhou, Suzhou–Nantong); no ferry, rail, water or air
links. This is a simulation topology, not a navigation dataset or exhaustive
survey of every possible intercity road. Boundaries do not generate edges.
