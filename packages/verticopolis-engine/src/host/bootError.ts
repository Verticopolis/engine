import type { PackageFailureDetail, PackageFailureKind } from "./enginePackageCheck.ts";

/**
 * Why an engine failed to boot or stopped, as codes from closed lists, and the
 * error a loader throws when it already knows the code. Split out of the
 * game's engineBoot.ts (Verticopolis/components, src/wasmhost/engineBoot.ts)
 * so the loaders here can throw it; the boot screen, its state and its copy
 * stay with the game.
 */

/**
 * Why the engine fell back (or, in a wasm-only build, stopped), as a code from
 * a closed list. The code is what analytics and the host status report
 * (`engine_status`, `docs/analytics-data-map.md`); the message beside it stays
 * on the console and the boot state and is never put in that event, since an
 * engine message can quote a tower name or a path.
 *
 * Boot sends the first six, plus `other` for a boot step outside the engine
 * that failed after the host took the tower (the host is let go). The
 * runtime fallback (#874) sends the trap codes for a running engine's fault
 * in a command (the undo capture's save just before an edit included), a
 * tick, a frame sync or the event drain, or an undo or redo, and `other` for
 * a save or the reads when a tower is let go. A trap in the engine's save
 * writer (#1205) is `trap-save`, and one in the slot census that loads a save
 * into a short-lived engine (#1612) is `trap-census`.
 */
export const FALLBACK_REASONS = ["none", "boot-timeout", "boot-error", "package-mismatch", "unverifiable", "attach-refused", "trap-command", "trap-tick", "trap-undo", "trap-save", "trap-census", "other"] as const;
export type FallbackReason = (typeof FALLBACK_REASONS)[number];

/** Where a fallback happened: loading the package, starting the host on the
 *  first tower, or a later tower the host was handed (a load or a new game,
 *  or an undo restore). Closed like {@link FALLBACK_REASONS}. A running
 *  engine's fault (#874) takes the stage of how its tower was hosted, so the
 *  trap code says which path failed and the stage on which kind of tower. */
export const FALLBACK_STAGES = ["none", "load", "start", "adopt", "undo"] as const;
export type FallbackStage = (typeof FALLBACK_STAGES)[number];

/** Why the byte check refused the engine package, as plain data (so a worker
 *  can send it): the kind, the file, both package ids and whether a service
 *  worker controlled the page (#985). Hex ids and codes only; the boot state
 *  carries it for tooling and support, and analytics never does. */
export type PackageFailure = PackageFailureDetail & { kind: PackageFailureKind };

/** A boot failure that already knows its code: the load's time limit, a
 *  package that does not match this shell. Anything else on the way to a
 *  running engine is `boot-error` (see {@link bootFailureReason}). */
export class EngineBootError extends Error {
  readonly reason: FallbackReason;
  readonly package?: PackageFailure;
  constructor(reason: FallbackReason, message: string, pkg?: PackageFailure) {
    super(message);
    this.name = "EngineBootError";
    this.reason = reason;
    if (pkg) this.package = pkg;
  }
}
