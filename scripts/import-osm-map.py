#!/usr/bin/env python3
"""Extract selected OSM administrative polygons and streets into a MeetAgain map directory."""

import argparse
import gzip
import json
import re
from contextlib import ExitStack
from pathlib import Path
from tempfile import TemporaryDirectory

import osmium
from shapely import from_wkb, make_valid, prepare
from shapely.geometry import LineString, MultiPolygon, Point, mapping
from shapely.strtree import STRtree
from shapely.validation import explain_validity


HIGHWAYS = {
    "major": ("motorway", "motorway_link", "trunk", "trunk_link", "primary", "primary_link"),
    "arterial": ("secondary", "secondary_link", "tertiary", "tertiary_link"),
    "local": ("unclassified", "residential", "living_street", "service", "pedestrian", "track", "road"),
}
ROAD_CLASS = {highway: label for label, highways in HIGHWAYS.items() for highway in highways}
CITY_ID = re.compile(r"[a-z0-9][a-z0-9_-]{0,31}")


def city_argument(value):
    city_id, separator, relation = value.partition("=")
    if not separator or not CITY_ID.fullmatch(city_id) or not relation.isdecimal() or int(relation) <= 0:
        raise argparse.ArgumentTypeError("expected CITY_ID=OSM_RELATION_ID")
    return city_id, int(relation)


def parts(geometry, kind):
    if geometry.is_empty:
        return
    if geometry.geom_type == kind:
        yield geometry
    elif hasattr(geometry, "geoms"):
        for child in geometry.geoms:
            yield from parts(child, kind)


def relation_metadata(source, selected):
    records = {}
    for relation in osmium.FileProcessor(source, osmium.osm.RELATION):
        if relation.id not in selected:
            continue
        tags = relation.tags
        if tags.get("boundary") != "administrative":
            raise ValueError(f"relation {relation.id} is not an administrative boundary")
        name = tags.get("name:zh-Hans") or tags.get("name:zh") or tags.get("name")
        if not name:
            raise ValueError(f"relation {relation.id} has no name")
        members = {
            member.role: member.ref
            for member in relation.members
            if member.type == "n" and member.role in ("admin_centre", "label")
        }
        records[relation.id] = name, members.get("admin_centre", members.get("label"))
    if missing := selected - records.keys():
        raise ValueError(f"relations missing from input: {sorted(missing)}")
    return records


def boundaries(source, records):
    filter_ids = osmium.filter.IdFilter(records).enable_for(osmium.osm.RELATION)
    processor = osmium.FileProcessor(source).with_areas(filter_ids)
    factory = osmium.geom.WKBFactory()
    result = {}
    for entity in processor:
        if not isinstance(entity, osmium.osm.Area) or entity.from_way():
            continue
        relation = entity.orig_id()
        if relation not in records:
            continue
        shape = from_wkb(bytes.fromhex(factory.create_multipolygon(entity)))
        if not shape.is_valid:
            shape = MultiPolygon(list(parts(make_valid(shape), "Polygon")))
        if shape.is_empty or not shape.is_valid:
            raise ValueError(f"relation {relation} has an unusable polygon: {explain_validity(shape)}")
        _, center_id = records[relation]
        center = None
        if center_id is not None:
            location = processor.node_location_storage.get(center_id)
            if location.valid():
                point = Point(location.lon, location.lat)
                if shape.covers(point):
                    center = point
        result[relation] = (shape, center if center is not None else shape.representative_point())
    if missing := records.keys() - result.keys():
        raise ValueError(f"could not assemble boundaries for relations {sorted(missing)}; use a larger extract")
    return result


def clipped_streets(source, polygons, writers):
    tree = STRtree(polygons)
    for polygon in polygons:
        prepare(polygon)
    highway_filter = osmium.filter.TagFilter(
        *(("highway", highway) for highway in ROAD_CLASS)
    )
    processor = osmium.FileProcessor(source, osmium.osm.NODE | osmium.osm.WAY).with_locations()
    processor.with_filter(osmium.filter.EntityFilter(osmium.osm.WAY)).with_filter(highway_filter)
    for way in processor:
        if way.tags.get("area") == "yes":
            continue
        if len(way.nodes) < 2 or any(not node.location.valid() for node in way.nodes):
            continue
        line = LineString([(node.lon, node.lat) for node in way.nodes])
        if line.length == 0:
            continue
        for index in tree.query(line):
            polygon = polygons[index]
            if not polygon.intersects(line):
                continue
            clipped = line if polygon.covers(line) else line.intersection(polygon)
            highway = way.tags["highway"]
            properties = {
                "osm_id": way.id,
                "highway": highway,
                "class": ROAD_CLASS[highway],
            }
            for key in ("name", "ref", "bridge", "tunnel", "oneway"):
                if value := way.tags.get(key):
                    properties[key] = value
            for segment in parts(clipped, "LineString"):
                coords = []
                for x, y in segment.coords:
                    point = [round(x, 7), round(y, 7)]
                    if not coords or coords[-1] != point:
                        coords.append(point)
                if len(coords) >= 2:
                    writers[index].write({
                        "type": "Feature",
                        "properties": properties,
                        "geometry": {"type": "LineString", "coordinates": coords},
                    })


class StreetWriter:
    def __init__(self, stream):
        self.stream = stream
        self.count = 0
        self.stream.write(b'{"type":"FeatureCollection","features":[\n')

    def write(self, feature):
        if self.count:
            self.stream.write(b",\n")
        self.stream.write(json.dumps(feature, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))
        self.count += 1

    def finish(self):
        self.stream.write(b"\n]}\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pbf", type=Path)
    parser.add_argument("--map-dir", type=Path, required=True)
    parser.add_argument("--city", type=city_argument, action="append", required=True,
                        help="CITY_ID=OSM_RELATION_ID; repeat to import several cities")
    parser.add_argument("--source-url", required=True, help="URL of the downloaded OSM extract")
    args = parser.parse_args()

    city_ids, relation_ids = zip(*args.city)
    if len(set(city_ids)) != len(city_ids) or len(set(relation_ids)) != len(relation_ids):
        parser.error("city IDs and relation IDs must each be unique")
    records = relation_metadata(args.pbf, set(relation_ids))
    shapes = boundaries(args.pbf, records)
    reader = osmium.io.Reader(args.pbf)
    timestamp = reader.header().get("osmosis_replication_timestamp") or None
    reader.close()

    args.map_dir.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(prefix=".osm-import-", dir=args.map_dir) as temporary:
        staging = Path(temporary)
        with ExitStack() as stack:
            writers = []
            for city_id in city_ids:
                folder = staging / city_id
                folder.mkdir()
                stream = stack.enter_context((folder / "streets.geojson.gz").open("wb"))
                zipped = stack.enter_context(gzip.GzipFile(filename="", mode="wb", fileobj=stream, mtime=0))
                writers.append(StreetWriter(zipped))
            clipped_streets(args.pbf, [shapes[relation][0] for relation in relation_ids], writers)
            for city_id, relation, writer in zip(city_ids, relation_ids, writers):
                writer.finish()
                if not writer.count:
                    raise ValueError(f"no street features inside {city_id}")
                shape, center = shapes[relation]
                boundary = {
                    "type": "Feature",
                    "properties": {
                        "city_id": city_id,
                        "name": records[relation][0],
                        "center": [round(center.x, 7), round(center.y, 7)],
                        "osm_relation": relation,
                        "source_url": args.source_url,
                        "osm_timestamp": timestamp,
                    },
                    "geometry": mapping(shape),
                }
                (staging / city_id / "boundary.geojson").write_text(
                    json.dumps(boundary, ensure_ascii=False, separators=(",", ":")) + "\n",
                    encoding="utf-8", newline="\n",
                )
                print(f"{city_id} {records[relation][0]}: {writer.count:,} street features")
        for city_id in city_ids:
            folder = args.map_dir / "cities" / city_id
            folder.mkdir(parents=True, exist_ok=True)
            for filename in ("boundary.geojson", "streets.geojson.gz"):
                (staging / city_id / filename).replace(folder / filename)


if __name__ == "__main__":
    main()
