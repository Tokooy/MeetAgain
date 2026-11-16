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
import type { Graph, Simulation } from "../types";
import "maplibre-gl/dist/maplibre-gl.css";
import mapWorkerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
maplibregl.setWorkerUrl(mapWorkerUrl);

interface Props {
  graph: Graph;
  result?: Simulation;
}
const empty: FeatureCollection = { type: "FeatureCollection", features: [] };
const colors = {
  dark: {
    fill: "#203a40",
    stroke: "#49646a",
    road: "#64878b",
    node: "#a9c6c9",
    trail: "#87dfc9",
  },
  light: {
    fill: "#d9e6e3",
    stroke: "#849f9a",
    road: "#7c9690",
    node: "#365b55",
    trail: "#087969",
  },
};
function recolor(map: LibreMap, theme: "dark" | "light") {
  const p = colors[theme];
  map.setPaintProperty("boundaries-fill", "fill-color", p.fill);
  map.setPaintProperty("boundaries-outline", "line-color", p.stroke);
  map.setPaintProperty("roads", "line-color", p.road);
  map.setPaintProperty("cities", "circle-color", p.node);
  map.setPaintProperty("trail", "line-color", p.trail);
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
  const animation = useRef(0);
  const previous = useRef(new Map<number, [number, number]>());
  const [ready, setReady] = useState(false);
  const [error, setError] = useState(false);
  const { tick, theme, selectedAgent } = useView();
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
    const onContextLost = () => setError(true);
    try {
      instance = new maplibregl.Map({
        container: container.current,
        style: { version: 8, sources: {}, layers: [] },
        center: [119.2, 33],
        zoom: 5.6,
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
        const roads: FeatureCollection<LineString> = {
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
        instance.addSource("roads", { type: "geojson", data: roads });
        instance.addLayer({
          id: "roads",
          type: "line",
          source: "roads",
          paint: {
            "line-color": "#64878b",
            "line-width": 1.3,
            "line-opacity": 0.65,
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
          label.textContent = city.name.replace("市", "");
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
        recolor(instance, useView.getState().theme);
        instance.fitBounds(modelBounds(graph), {
          padding: { top: 72, bottom: 96, left: 44, right: 44 },
          duration: 0,
        });
        setReady(true);
      });
    } catch {
      setError(true);
      return;
    }
    const observer = new ResizeObserver(() => instance.resize());
    observer.observe(container.current);
    return () => {
      cancelAnimationFrame(animation.current);
      observer.disconnect();
      markers.forEach((m) => m.remove());
      instance
        .getCanvas()
        .removeEventListener("webglcontextlost", onContextLost);
      instance.remove();
      map.current = null;
      setReady(false);
    };
  }, [graph]);
  useEffect(() => {
    if (ready && map.current) recolor(map.current, theme);
  }, [theme, ready]);
  useEffect(() => {
    const instance = map.current;
    if (!ready || !instance) return;
    cancelAnimationFrame(animation.current);
    const nodes = new Map(graph.nodes.map((c) => [c.id, c.center]));
    const destination = new Map<number, [number, number]>();
    const features: FeatureCollection<Point> = {
      type: "FeatureCollection",
      features:
        result?.agents.map((a) => {
          const center = nodes.get(
            a.trajectory[Math.min(tick, a.trajectory.length - 1)].city_id,
          )!;
          const angle = a.id * 2.39996;
          const radius = 0.016 + (a.id % 5) * 0.006;
          const position: [number, number] = [
            center[0] + Math.cos(angle) * radius,
            center[1] + Math.sin(angle) * radius,
          ];
          destination.set(a.id, position);
          return {
            type: "Feature",
            properties: { id: a.id, selected: a.id === selectedAgent },
            geometry: { type: "Point", coordinates: position },
          };
        }) ?? [],
    };
    const start = performance.now();
    function frame(now: number) {
      const progress = reduced.current ? 1 : Math.min(1, (now - start) / 200);
      const ease = progress * progress * (3 - 2 * progress);
      for (const feature of features.features) {
        const id = feature.properties?.id as number;
        const to = destination.get(id)!;
        const from = previous.current.get(id) ?? to;
        feature.geometry.coordinates = [
          from[0] + (to[0] - from[0]) * ease,
          from[1] + (to[1] - from[1]) * ease,
        ];
      }
      (instance!.getSource("agents") as GeoJSONSource).setData(features);
      if (progress < 1) animation.current = requestAnimationFrame(frame);
      else previous.current = destination;
    }
    animation.current = requestAnimationFrame(frame);
    const agent = result?.agents.find((a) => a.id === selectedAgent);
    const trail: FeatureCollection<LineString> = {
      type: "FeatureCollection",
      features: [],
    };
    if (agent && tick > 0) {
      const positions = agent.trajectory.slice(
        Math.max(0, tick - 12),
        tick + 1,
      );
      for (let i = 1; i < positions.length; i++) {
        if (positions[i - 1].city_id === positions[i].city_id) continue;
        trail.features.push({
          type: "Feature",
          properties: { opacity: 0.15 + (0.65 * i) / positions.length },
          geometry: {
            type: "LineString",
            coordinates: [
              nodes.get(positions[i - 1].city_id)!,
              nodes.get(positions[i].city_id)!,
            ],
          },
        });
      }
    }
    (instance.getSource("trail") as GeoJSONSource).setData(trail);
    return () => cancelAnimationFrame(animation.current);
  }, [tick, result, ready, selectedAgent, graph]);
  return (
    <div className="map-shell">
      <div
        className="map-canvas"
        ref={container}
        role="img"
        aria-label="江苏十三市边界、道路网络和个体位置地图"
        data-testid="city-map"
        data-ready={ready}
      />
      <div className="map-heading">
        <span className="live-dot" />
        JIANGSU · 江苏省<span className="map-model">城市级道路模型</span>
      </div>
      <div className="map-compass" aria-hidden="true">
        <span>N</span>↑
      </div>
      <button
        className="icon-button fit-map"
        aria-label="显示完整江苏地图"
        disabled={!ready}
        onClick={() =>
          map.current?.fitBounds(modelBounds(graph), {
            padding: { top: 72, bottom: 96, left: 44, right: 44 },
            duration: reduced.current ? 0 : 250,
          })
        }
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
          地图暂不可用，请检查浏览器 WebGL 支持。轨迹和事件仍可查看。
        </div>
      )}
      <div className="map-legend">
        <span>
          <i className="legend-node" />
          城市中心
        </span>
        <span>
          <i className="legend-road" />
          道路连接
        </span>
        {result && (
          <span>
            <i className="legend-agent" />
            个体位置
          </span>
        )}
      </div>
      <span className="map-coordinate-note">
        示意边界 · 点位偏移仅用于区分个体
      </span>
    </div>
  );
}
