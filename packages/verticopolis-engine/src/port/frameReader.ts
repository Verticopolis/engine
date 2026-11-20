import type { ElevatorQueueView, HeatCell, HeatmapMode, QueueLanding } from "../model/engineTypes.ts";
import type { GameMode, SerializedView, WeatherKind } from "../model/types.ts";
import { ClockReads } from "./clockView.ts";
import type { EnginePort, PortRecord } from "./enginePort.ts";
import type { CrowdView, TowerView } from "./view.ts";
import type { ViewReader } from "./viewReader.ts";

/** The tower a frame reader hands out: the view reader's tower plus two port
 *  reads a frontend makes on a scene sync. */
export interface FrameTower extends TowerView {
  /** Working garage spaces by unit id, decoded on each call. */
  functionalParkingSet(): ReadonlySet<number>;
  /** The engine's lobby heights, as the port answers them (`lobbyHeights`):
   *  the raw record, for a frontend to lay out its tall lobby halls. Read once
   *  per structure pass, never per frame. */
  lobbyHeights(): PortRecord;
}

/**
 * A frontend's reads of one tower's frame, over the port: the view reads come
 * from the shared `ViewReader`, the rest from the port's render queries. The
 * frame loop updates the reader before a frontend reads it. It is typed with
 * the engine's own shapes and knows nothing of any renderer, so a renderer
 * that declares its own input types takes it as it is, with no copy.
 *
 * The queue projection is decoded once per tower, game minute and revision
 * (the crowd moves only on a step, and a step moves the clock), so a frame
 * between steps reads the kept one without asking the port. The parking set is
 * decoded per call, once per scene sync. The clock is one reused `ClockReads`.
 */
export class FrameReader {
  private readonly clockReads = new ClockReads();
  readonly tower: FrameTower;
  /** The crowd, plus the elevator queue projection. `queueView` takes no
   *  argument: the reader answers for its own tower. */
  readonly crowd: CrowdView & { readonly queueView: () => ElevatorQueueView };
  private readonly heatModes: readonly HeatmapMode[];
  private readonly heatTints: readonly string[];
  private queueKey = { gen: NaN, minutes: NaN, revision: NaN };
  private queue: ElevatorQueueView = { landings: new Map(), boarded: new Map() };

  private readonly port: EnginePort;
  private readonly reader: ViewReader;

  constructor(
    port: EnginePort,
    reader: ViewReader,
  ) {
    this.port = port;
    this.reader = reader;
    const l = JSON.parse(port.viewLayout()) as { heatModes: HeatmapMode[]; heatTints: string[] };
    this.heatModes = l.heatModes;
    this.heatTints = l.heatTints;
    const tower = Object.create(reader.tower) as FrameTower;
    tower.functionalParkingSet = () => new Set(port.parkingSet().subarray(1));
    tower.lobbyHeights = () => port.lobbyHeights();
    this.tower = tower;
    this.crowd = { people: reader.crowd.people, queueView: () => this.queueView() };
  }

  get mode(): GameMode {
    return this.reader.header.mode;
  }
  get rng(): { readonly initialSeed: number } {
    return { initialSeed: this.reader.header.sceneSeed };
  }
  get view(): SerializedView | null {
    return this.reader.header.camera;
  }
  get weather(): WeatherKind {
    return this.reader.header.weather;
  }
  get population(): number {
    return this.reader.header.population;
  }
  get santaFxSeq(): number {
    return this.reader.header.fx.santaSeq;
  }
  get vipFxSeq(): number {
    return this.reader.header.fx.vipSeq;
  }
  get explosionFx() {
    return this.reader.header.fx.explosion;
  }
  get thiefFx() {
    return this.reader.header.fx.thief;
  }
  get treasureFx() {
    return this.reader.header.fx.treasure;
  }
  get bombSearch() {
    return this.reader.header.bombSearch;
  }
  get fireResponse() {
    return this.reader.header.fireResponse;
  }

  /** The reader's clock, set from its header on each read without allocating. */
  get clock(): ClockReads {
    return this.clockReads.set(this.reader.header);
  }

  congestion(): number {
    return this.port.congestion();
  }
  peakCongestion(): number {
    return this.port.hotspot()[0];
  }
  floorHeatmap(mode: HeatmapMode): HeatCell[] {
    const r = this.port.heatCells(this.heatModes.indexOf(mode));
    const cells: HeatCell[] = [];
    for (let i = 0; i + 5 <= r.length; i += 5) {
      const cell: HeatCell = { floor: r[i], minX: r[i + 1], maxX: r[i + 2], severity: r[i + 3] };
      if (r[i + 4] > 0) cell.tint = this.heatTints[r[i + 4]] as HeatCell["tint"];
      cells.push(cell);
    }
    return cells;
  }
  parkingUsage(spots: number): number {
    return this.port.parkingUsage(spots);
  }
  recyclingFill(): number {
    return this.port.recyclingFill();
  }
  positionReachable(floor: number, x: number): boolean {
    return this.port.positionReachable(floor, x);
  }

  private queueView(): ElevatorQueueView {
    const h = this.reader.header;
    const k = this.queueKey;
    const gen = this.port.generation();
    if (k.gen === gen && k.minutes === h.minutes && k.revision === h.revision) return this.queue;
    const r = this.port.queueView();
    const landings = new Map<number, Map<number, QueueLanding>>();
    const boarded = new Map<number, Map<number, number>>();
    let at = 2;
    for (let i = 0; i < r[0]; i++, at += 4) {
      let floors = landings.get(r[at]);
      if (!floors) landings.set(r[at], (floors = new Map()));
      floors.set(r[at + 1], { count: r[at + 2], tier: r[at + 3] as QueueLanding["tier"] });
    }
    for (let i = 0; i < r[1]; i++, at += 3) {
      let cars = boarded.get(r[at]);
      if (!cars) boarded.set(r[at], (cars = new Map()));
      cars.set(r[at + 1], r[at + 2]);
    }
    k.gen = gen;
    k.minutes = h.minutes;
    k.revision = h.revision;
    this.queue = { landings, boarded };
    return this.queue;
  }
}

