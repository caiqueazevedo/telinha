export type Point = { x: number; y: number };
export type Rect = { x: number; y: number; width: number; height: number };

/** Smallest side the backend accepts for a region (matches `region::MIN_SIDE`). */
export const MIN_SIDE = 16;

/** Turns a drag in any direction into a rectangle, scaled from CSS to physical pixels. */
export function normalizeDrag(start: Point, end: Point, scale = 1): Rect {
  return {
    x: Math.round(Math.min(start.x, end.x) * scale),
    y: Math.round(Math.min(start.y, end.y) * scale),
    width: Math.round(Math.abs(end.x - start.x) * scale),
    height: Math.round(Math.abs(end.y - start.y) * scale),
  };
}

export function isUsable(rect: Rect): boolean {
  return rect.width >= MIN_SIDE && rect.height >= MIN_SIDE;
}
