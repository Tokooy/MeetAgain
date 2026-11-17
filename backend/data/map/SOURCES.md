# Bundled map provenance

This directory contains a regional extract of **OpenStreetMap data**, © OpenStreetMap
contributors, under the [Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
See [OpenStreetMap copyright and attribution](https://www.openstreetmap.org/copyright).
The license covers the data in `cities/`.

Source: [Geofabrik Jiangsu dated PBF](https://download.geofabrik.de/asia/china/jiangsu-261116.osm.pbf)
([Jiangsu download page](https://download.geofabrik.de/asia/china/jiangsu.html)).
PBF replication timestamp: `2026-11-16T21:22:36Z`.
SHA-256: `58ab61bfd510d707865e0c81ecaccb3e25bd790c22a6279ac6809e225a1b3f8b`.

| ID | City | Administrative relation | Clipped road line features |
| --- | --- | --- | ---: |
| 320100 | 南京市 | [2131524](https://www.openstreetmap.org/relation/2131524) | 46,963 |
| 321100 | 镇江市 | [3429680](https://www.openstreetmap.org/relation/3429680) | 19,997 |
| 321000 | 扬州市 | [4430826](https://www.openstreetmap.org/relation/4430826) | 24,589 |

`scripts/import-osm-map.py` assembles the selected administrative polygons
from the PBF and clips highway centerlines to each polygon. It keeps the
OpenStreetMap way ID, classification, and available name, reference, bridge,
tunnel, and oneway tags. Clipping can yield multiple lines for one way. Polygon
features and street coordinates are WGS84 longitude/latitude. Only the three
listed cities are distributed. See the project README for the import command.

`map.json` lists the active city IDs and the simulation's undirected city
links. Link costs use spherical distance between representative city points.