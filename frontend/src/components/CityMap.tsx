import { useEffect, useRef, useState } from "react";
import * as maplibregl from "maplibre-gl";
import type {
  GeoJSONSource,
  Map as LibreMap,
  LngLatBoundsLike,
} from "maplibre-gl";
import type { FeatureCollection, Point, LineString } from "geojson";
import { Expand, MapPin } from "lucide-react";
import { useView } from "../store";
import type { City, Graph, Simulation } from "../types";
import "maplibre-gl/dist/maplibre-gl.css";
import mapWorkerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
maplibregl.setWorkerUrl(mapWorkerUrl);

interface Props {
  graph: Graph;
  result?: Simulation;
}
const empty: FeatureCollection = { type: "FeatureCollection", features: [] };
const streetLayers = {
  local: {
    minZoom: 10.5,
    overviewWidth: 0.65,
    detailWidth: 1.35,
    opacity: 0.51,
  },
  arterial: {
    minZoom: 7.5,
    overviewWidth: 0.75,
    detailWidth: 2.1,
    opacity: 0.73,
  },
  major: { minZoom: 0, overviewWidth: 1.05, detailWidth: 3.0, opacity: 0.85 },
} as const;
type StreetClass = keyof typeof streetLayers;
const streetClasses = Object.keys(streetLayers) as StreetClass[];
const streetLayerId = (cityId: string, roadClass: StreetClass) =>
  `street-${roadClass}-${cityId}`;
const fitPadding = { top: 86, bottom: 86, left: 28, right: 28 };
const colors = {
  dark: {
    fill: "#1c3338",
    stroke: "#607c7a",
    link: "#8dc3b5",
    streets: { major: "#dbaf6e", arterial: "#7da6a0", local: "#6f8b88" },
    node: "#a9c6c9",
    trail: "#87dfc9",
  },
  light: {
    fill: "#e2ece7",
    stroke: "#72928a",
    link: "#3a7067",
    streets: { major: "#ad7427", arterial: "#617b74", local: "#6f8882" },
    node: "#365b55",
    trail: "#087969",
  },
};
function recolor(map: LibreMap, theme: "dark" | "light", cities: City[]) {
  const p = colors[theme];
  map.setPaintProperty("boundaries-fill", "fill-color", p.fill);
  map.setPaintProperty("boundaries-outline", "line-color", p.stroke);
  map.setPaintProperty("city-links", "line-color", p.link);
  for (const city of cities)
    for (const roadClass of streetClasses)
      map.setPaintProperty(
        streetLayerId(city.id, roadClass),
        "line-color",
        p.streets[roadClass],
      );
  map.setPaintProperty("cities", "circle-color", p.node);
  map.setPaintProperty("trail", "line-color", p.trail);
  map.setPaintProperty("residence", "circle-stroke-color", p.node);
  map.setPaintProperty(
    "agents",
    "circle-color",
    theme === "dark" ? "#8ce2c9" : "#087969",
  );
}
function modelBounds(graph: Graph): LngLatBoundsLike {
  const bounds = new maplibregl.LngLatBounds();
  function visit(
    coordinates: number[] | number[][] | number[][][] | number[][][][],
  ) {
    if (typeof coordinates[0] === "number")
      bounds.extend(coordinates as [number, number]);
    else
      for (const child of coordinates)
        visit(child as number[] | number[][] | number[][][]);
  }
  for (const f of graph.boundaries.features) visit(f.geometry.coordinates);
  return bounds;
}
export default function CityMap({ graph, result }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const map = useRef<LibreMap | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState(false);
  const [focusedCity, setFocusedCity] = useState("");
  const { tick, theme, selectedAgent, focusPoint } = useView();
  const reduced = useRef(
    window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useEffect(() => {
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => {
      reduced.current = media.matches;
    };
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    if (!container.current) return;
    let instance: LibreMap;
    const markers: maplibregl.Marker[] = [];
    let roadPopup: maplibregl.Popup | undefined;
    const roadLayerIds = graph.nodes.flatMap((city) =>
      streetClasses.map((roadClass) => streetLayerId(city.id, roadClass)),
    );
    const onContextLost = () => setError(true);
    try {
      instance = new maplibregl.Map({
        container: container.current,
        style: { version: 8, sources: {}, layers: [] },
        bounds: modelBounds(graph),
        fitBoundsOptions: { padding: fitPadding, duration: 0 },
        renderWorldCopies: false,
        attributionControl: false,
        dragRotate: false,
        touchPitch: false,
        canvasContextAttributes: { antialias: true },
      });
      map.current = instance;
      instance.getCanvas().addEventListener("webglcontextlost", onContextLost);
      instance.on("error", () => setError(true));
      instance.on("idle", () => {
        if (
          container.current?.dataset.streetsRendered !== "true" &&
          roadLayerIds.every((id) => instance.getLayer(id)) &&
          instance.queryRenderedFeatures({ layers: roadLayerIds }).length > 0
        ) {
          container.current?.setAttribute("data-streets-rendered", "true");
        }
        if (
          container.current?.dataset.rendered !== "true" &&
          instance.getLayer("boundaries-fill") &&
          instance.queryRenderedFeatures({ layers: ["boundaries-fill"] })
            .length > 0
        )
          container.current?.setAttribute("data-rendered", "true");
      });
      instance.on("load", () => {
        const lookup = new Map(graph.nodes.map((n) => [n.id, n.center]));
        instance.addSource("boundaries", {
          type: "geojson",
          data: graph.boundaries,
        });
        instance.addLayer({
          id: "boundaries-fill",
          type: "fill",
          source: "boundaries",
          paint: { "fill-color": "#203a40", "fill-opacity": 0.92 },
        });
        instance.addLayer({
          id: "boundaries-outline",
          type: "line",
          source: "boundaries",
          paint: { "line-color": "#49646a", "line-width": 1 },
        });
        const palette = colors[useView.getState().theme];
        for (const city of graph.nodes) {
          instance.addSource(`street-data-${city.id}`, {
            type: "geojson",
            data: `/api/map/roads/${city.id}`,
            maxzoom: 14,
          });
        }
        for (const roadClass of streetClasses) {
          const { minZoom, overviewWidth, detailWidth, opacity } =
            streetLayers[roadClass];
          for (const city of graph.nodes) {
            instance.addLayer({
              id: streetLayerId(city.id, roadClass),
              type: "line",
              source: `street-data-${city.id}`,
              minzoom: minZoom,
              filter: ["==", ["get", "class"], roadClass],
              layout: { "line-join": "round", "line-cap": "round" },
              paint: {
                "line-color": palette.streets[roadClass],
                "line-width": [
                  "interpolate",
                  ["linear"],
                  ["zoom"],
                  8,
                  overviewWidth,
                  14,
                  detailWidth,
                ],
                "line-opacity": opacity,
              },
            });
          }
        }
        const links: FeatureCollection<LineString> = {
          type: "FeatureCollection",
          features: graph.edges.map((e) => ({
            type: "Feature",
            properties: {},
            geometry: {
              type: "LineString",
              coordinates: [lookup.get(e.source)!, lookup.get(e.target)!],
            },
          })),
        };
        instance.addSource("city-links", { type: "geojson", data: links });
        instance.addLayer({
          id: "city-links",
          type: "line",
          source: "city-links",
          paint: {
            "line-color": palette.link,
            "line-width": 1.6,
            "line-opacity": 0.82,
            "line-dasharray": [3, 3],
          },
        });
        instance.addSource("trail", { type: "geojson", data: empty });
        instance.addLayer({
          id: "trail",
          type: "line",
          source: "trail",
          paint: {
            "line-color": "#87dfc9",
            "line-width": 2.5,
            "line-opacity": ["get", "opacity"],
            "line-dasharray": [2, 1.5],
          },
        });
        const cities: FeatureCollection<Point> = {
          type: "FeatureCollection",
          features: graph.nodes.map((n) => ({
            type: "Feature",
            properties: {},
            geometry: { type: "Point", coordinates: n.center },
          })),
        };
        instance.addSource("cities", { type: "geojson", data: cities });
        instance.addLayer({
          id: "cities",
          type: "circle",
          source: "cities",
          paint: { "circle-radius": 3, "circle-color": "#a9c6c9" },
        });
        instance.addSource("residence", { type: "geojson", data: empty });
        instance.addLayer({
          id: "residence",
          type: "circle",
          source: "residence",
          paint: {
            "circle-radius": 7,
            "circle-opacity": 0,
            "circle-stroke-width": 2,
            "circle-stroke-color": "#365b55",
          },
        });
        instance.addSource("agents", { type: "geojson", data: empty });
        instance.addLayer({
          id: "agent-halos",
          type: "circle",
          source: "agents",
          paint: {
            "circle-radius": [
              "case",
              ["boolean", ["get", "selected"], false],
              12,
              8,
            ],
            "circle-color": "#60c9ad",
            "circle-opacity": 0.12,
          },
        });
        instance.addLayer({
          id: "agents",
          type: "circle",
          source: "agents",
          paint: {
            "circle-radius": [
              "case",
              ["boolean", ["get", "selected"], false],
              5,
              3.5,
            ],
            "circle-color": "#8ce2c9",
            "circle-stroke-color": "#163e36",
            "circle-stroke-width": 1,
          },
        });
        for (const city of graph.nodes) {
          const label = document.createElement("span");
          label.className = "city-name";
          label.textContent = city.name;
          markers.push(
            new maplibregl.Marker({
              element: label,
              anchor: "top",
              offset: [0, 9],
            })
              .setLngLat(city.center)
              .addTo(instance),
          );
        }
        instance.on("click", "agents", (e) => {
          const id = e.features?.[0]?.properties?.id as unknown;
          if (typeof id === "number") useView.getState().selectAgent(id);
        });
        instance.on("mouseenter", "agents", () => {
          instance.getCanvas().style.cursor = "pointer";
        });
        instance.on("mouseleave", "agents", () => {
          instance.getCanvas().style.cursor = "";
        });
        instance.on("click", (event) => {
          if (
            instance.queryRenderedFeatures(event.point, { layers: ["agents"] })
              .length
          )
            return;
          const properties = instance.queryRenderedFeatures(event.point, {
            layers: roadLayerIds,
          })[0]?.properties;
          if (!properties) return;
          roadPopup?.remove();
          const label =
            [properties.name, properties.ref].filter(Boolean).join(" · ") ||
            `OpenStreetMap 路段 #${properties.osm_id}`;
          roadPopup = new maplibregl.Popup({
            offset: 8,
            className: "street-popup",
          })
            .setLngLat(event.lngLat)
            .setText(label)
            .addTo(instance);
        });
        recolor(instance, useView.getState().theme, graph.nodes);
        setReady(true);
      });
    } catch {
      setError(true);
      return;
    }
    const observer = new ResizeObserver(() => instance.resize());
    observer.observe(container.current);
    return () => {
      observer.disconnect();
      markers.forEach((m) => m.remove());
      roadPopup?.remove();
      instance
        .getCanvas()
        .removeEventListener("webglcontextlost", onContextLost);
      instance.remove();
      map.current = null;
      setReady(false);
    };
  }, [graph]);
  useEffect(() => {
    if (ready && map.current) recolor(map.current, theme, graph.nodes);
  }, [theme, ready, graph.nodes]);
  useEffect(() => {
    const instance = map.current;
    if (!ready || !instance) return;
    const features: FeatureCollection<Point> = {
      type: "FeatureCollection",
      features:
        result?.agents.map((agent) => ({
          type: "Feature",
          properties: { id: agent.id, selected: agent.id === selectedAgent },
          geometry: {
            type: "Point",
            coordinates:
              agent.trajectory[Math.min(tick, agent.trajectory.length - 1)]
                .coordinates,
          },
        })) ?? [],
    };
    (instance.getSource("agents") as GeoJSONSource).setData(features);

    const agent = result?.agents.find((a) => a.id === selectedAgent);
    const residence: FeatureCollection<Point> = {
      type: "FeatureCollection",
      features: agent
        ? [
            {
              type: "Feature",
              properties: {},
              geometry: {
                type: "Point",
                coordinates: agent.residence_coordinates,
              },
            },
          ]
        : [],
    };
    (instance.getSource("residence") as GeoJSONSource).setData(residence);
    const trail: FeatureCollection<LineString> = {
      type: "FeatureCollection",
      features: [],
    };
    const positions =
      agent?.trajectory.slice(Math.max(0, tick - 12), tick + 1) ?? [];
    for (let i = 1; i < positions.length; i++) {
      const from = positions[i - 1].coordinates;
      const to = positions[i].coordinates;
      if (from[0] === to[0] && from[1] === to[1]) continue;
      trail.features.push({
        type: "Feature",
        properties: { opacity: 0.15 + (0.65 * i) / positions.length },
        geometry: { type: "LineString", coordinates: [from, to] },
      });
    }
    (instance.getSource("trail") as GeoJSONSource).setData(trail);
  }, [tick, result, ready, selectedAgent]);

  useEffect(() => {
    if (!ready || !focusPoint) return;
    setFocusedCity("");
    map.current?.easeTo({
      center: focusPoint,
      zoom: 17,
      duration: reduced.current ? 0 : 300,
    });
  }, [focusPoint, ready]);
  function focusCity(id: string) {
    setFocusedCity(id);
    const city = graph.nodes.find((node) => node.id === id);
    const duration = reduced.current ? 0 : 300;
    if (city)
      map.current?.easeTo({ center: city.center, zoom: 11.7, duration });
    else
      map.current?.fitBounds(modelBounds(graph), {
        padding: fitPadding,
        duration,
      });
  }

  return (
    <div className="map-shell">
      <div
        className="map-canvas"
        ref={container}
        role="img"
        aria-label="城市边界、街道和个体位置地图"
        data-testid="city-map"
        data-ready={ready}
      />
      <div className="map-heading">
        <span className="live-dot" />
        城市道路网
        <select
          aria-label="缩放到城市"
          disabled={!ready}
          value={focusedCity}
          onChange={(event) => focusCity(event.target.value)}
        >
          <option value="">全部城市</option>
          {graph.nodes.map((city) => (
            <option key={city.id} value={city.id}>
              {city.name}
            </option>
          ))}
        </select>
      </div>
      <div className="map-compass" aria-hidden="true">
        <span>N</span>↑
      </div>
      <button
        className="icon-button fit-map"
        aria-label="显示全部城市"
        disabled={!ready}
        onClick={() => focusCity("")}
      >
        <Expand size={16} />
      </button>
      {!result && (
        <div className="map-empty">
          <MapPin size={15} />
          <span>每个人的故事，从一座城市开始</span>
        </div>
      )}
      {error && (
        <div className="map-error" role="alert">
          地图加载失败。请刷新页面，或检查浏览器的 WebGL 支持。
        </div>
      )}
      <div className="map-legend">
        <span>
          <i className="legend-node" />
          城市中心
        </span>
        <span>
          <i className="legend-street" />
          实际道路
        </span>
        <span>
          <i className="legend-road" />
          模拟连线
        </span>
        {result && (
          <span>
            <i className="legend-agent" />
            当天位置
          </span>
        )}
      </div>
      <a
        className="map-attribution"
        href={graph.provenance.license_url}
        target="_blank"
        rel="noreferrer"
      >
        {graph.provenance.attribution}
      </a>
    </div>
  );
}
