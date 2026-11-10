const SIZE = 100;
const SHAPE_SAMPLES = 96;

interface Point {
  x: number;
  y: number;
}

interface ShapeVertex extends Point {
  radius?: number;
  smoothing?: number;
}

interface ShapeDef {
  points: ShapeVertex[];
  reps?: number;
  mirror?: boolean;
  scaleY?: number;
  scale?: number;
}

const CENTER: Point = { x: 0.5, y: 0.5 };

function regularVertices(count: number, radius: number, angleOffset = 0, rounding = 0): ShapeVertex[] {
  return Array.from({ length: count }, (_, index) => {
    const angle = angleOffset + (index / count) * Math.PI * 2;
    return {
      x: CENTER.x + Math.cos(angle) * radius,
      y: CENTER.y + Math.sin(angle) * radius,
      radius: rounding
    };
  });
}

function starVertices(
  count: number,
  innerRatio: number,
  rounding: number,
  angleOffset = -Math.PI / 2
): ShapeVertex[] {
  const vertices: ShapeVertex[] = [];
  for (let index = 0; index < count; index += 1) {
    const outer = angleOffset + (index / count) * Math.PI * 2;
    const inner = outer + Math.PI / count;
    vertices.push({
      x: CENTER.x + Math.cos(outer) * 0.5,
      y: CENTER.y + Math.sin(outer) * 0.5,
      radius: rounding
    });
    vertices.push({
      x: CENTER.x + Math.cos(inner) * 0.5 * innerRatio,
      y: CENTER.y + Math.sin(inner) * 0.5 * innerRatio,
      radius: rounding
    });
  }
  return vertices;
}

function ellipseVertices(count: number, rx: number, ry: number, rotation = 0): ShapeVertex[] {
  return Array.from({ length: count }, (_, index) => {
    const angle = (index / count) * Math.PI * 2;
    const x = Math.cos(angle) * rx;
    const y = Math.sin(angle) * ry;
    return {
      x: CENTER.x + x * Math.cos(rotation) - y * Math.sin(rotation),
      y: CENTER.y + x * Math.sin(rotation) + y * Math.cos(rotation)
    };
  });
}

const puffyPoints: ShapeVertex[] = [
  { x: 0.5, y: 0.053 },
  { x: 0.545, y: -0.04, radius: 0.405 },
  { x: 0.67, y: -0.035, radius: 0.426 },
  { x: 0.717, y: 0.066, radius: 0.574 },
  { x: 0.722, y: 0.128 },
  { x: 0.777, y: 0.002, radius: 0.36 },
  { x: 0.914, y: 0.149, radius: 0.66 },
  { x: 0.926, y: 0.289, radius: 0.66 },
  { x: 0.881, y: 0.346 },
  { x: 0.94, y: 0.344, radius: 0.126 },
  { x: 1.003, y: 0.437, radius: 0.255 }
];

const SHAPE_DEFS = {
  circle: { points: regularVertices(64, 0.5) },
  square: {
    points: [
      { x: 1, y: 1, radius: 0.3 },
      { x: 0, y: 1, radius: 0.3 },
      { x: 0, y: 0, radius: 0.3 },
      { x: 1, y: 0, radius: 0.3 }
    ]
  },
  slanted: {
    points: [
      { x: 0.926, y: 0.97, radius: 0.189, smoothing: 0.811 },
      { x: -0.021, y: 0.967, radius: 0.187, smoothing: 0.057 }
    ],
    reps: 2
  },
  arch: {
    points: [
      { x: 0.85, y: 0.85, radius: 0.02 },
      { x: 0.85, y: 0.5 },
      ...Array.from({ length: 16 }, (_, index) => {
        const angle = -(index / 15) * Math.PI;
        return {
          x: CENTER.x + Math.cos(angle) * 0.35,
          y: CENTER.y + Math.sin(angle) * 0.35
        };
      }),
      { x: 0.15, y: 0.85, radius: 0.02 }
    ]
  },
  semiCircle: {
    points: [
      { x: 0.8, y: 0.5, radius: 0.2 },
      { x: -0.8, y: 0.5, radius: 0.2 },
      { x: -0.8, y: -0.5, radius: 1 },
      { x: 0.8, y: -0.5, radius: 1 }
    ],
    scale: 0.9
  },
  oval: { points: ellipseVertices(64, 0.5, 0.32) },
  pill: {
    points: [
      { x: 0.95, y: 0.78, radius: 0.28 },
      { x: 0.05, y: 0.78, radius: 0.28 },
      { x: 0.05, y: 0.22, radius: 0.28 },
      { x: 0.95, y: 0.22, radius: 0.28 }
    ]
  },
  triangle: { points: regularVertices(3, 1, -Math.PI / 2, 0.2) },
  arrow: {
    points: [
      { x: 0.12, y: 0.35, radius: 0.03 },
      { x: 0.55, y: 0.35, radius: 0.03 },
      { x: 0.55, y: 0.12, radius: 0.03 },
      { x: 0.92, y: 0.5, radius: 0.03 },
      { x: 0.55, y: 0.88, radius: 0.03 },
      { x: 0.55, y: 0.65, radius: 0.03 },
      { x: 0.12, y: 0.65, radius: 0.03 }
    ]
  },
  fan: {
    points: [
      { x: 1.004, y: 1, radius: 0.148, smoothing: 0.417 },
      { x: 0, y: 1, radius: 0.151 },
      { x: 0, y: -0.003, radius: 0.148 },
      { x: 0.978, y: 0.02, radius: 0.803 }
    ]
  },
  diamond: {
    points: [
      { x: 0.5, y: 1.096, radius: 0.151, smoothing: 0.524 },
      { x: 0.04, y: 0.5, radius: 0.159 }
    ],
    reps: 2
  },
  clamShell: {
    points: [
      { x: 0.171, y: 0.841, radius: 0.159 },
      { x: -0.02, y: 0.5, radius: 0.14 },
      { x: 0.17, y: 0.159, radius: 0.159 }
    ],
    reps: 2
  },
  pentagon: {
    points: [
      { x: 0.5, y: -0.009, radius: 0.172 },
      { x: 1.03, y: 0.365, radius: 0.164 },
      { x: 0.828, y: 0.97, radius: 0.169 }
    ],
    mirror: true
  },
  gem: {
    points: [
      { x: 0.499, y: 1.023, radius: 0.241, smoothing: 0.778 },
      { x: -0.005, y: 0.792, radius: 0.208 },
      { x: 0.073, y: 0.258, radius: 0.228 },
      { x: 0.433, y: 0, radius: 0.491 }
    ],
    mirror: true
  },
  sunny: { points: starVertices(8, 0.8, 0.15) },
  verySunny: {
    points: [
      { x: 0.5, y: 1.08, radius: 0.085 },
      { x: 0.358, y: 0.843, radius: 0.085 }
    ],
    reps: 8
  },
  cookie4Sided: {
    points: [
      { x: 1.237, y: 1.236, radius: 0.258 },
      { x: 0.5, y: 0.918, radius: 0.233 }
    ],
    reps: 4
  },
  cookie6Sided: {
    points: [
      { x: 0.723, y: 0.884, radius: 0.394 },
      { x: 0.5, y: 1.099, radius: 0.398 }
    ],
    reps: 6
  },
  cookie7Sided: { points: starVertices(7, 0.75, 0.5) },
  cookie9Sided: { points: starVertices(9, 0.8, 0.5) },
  cookie12Sided: { points: starVertices(12, 0.8, 0.5) },
  clover4Leaf: {
    points: [
      { x: 0.5, y: 0.074 },
      { x: 0.725, y: -0.099, radius: 0.476 }
    ],
    reps: 4,
    mirror: true
  },
  clover8Leaf: {
    points: [
      { x: 0.5, y: 0.036 },
      { x: 0.758, y: -0.101, radius: 0.209 }
    ],
    reps: 8
  },
  burst: {
    points: [
      { x: 0.5, y: -0.006, radius: 0.006 },
      { x: 0.592, y: 0.158, radius: 0.006 }
    ],
    reps: 12
  },
  softBurst: {
    points: [
      { x: 0.193, y: 0.277, radius: 0.053 },
      { x: 0.176, y: 0.055, radius: 0.053 }
    ],
    reps: 10
  },
  boom: {
    points: [
      { x: 0.457, y: 0.296, radius: 0.007 },
      { x: 0.5, y: -0.051, radius: 0.007 }
    ],
    reps: 15
  },
  softBoom: {
    points: [
      { x: 0.733, y: 0.454 },
      { x: 0.839, y: 0.437, radius: 0.532 },
      { x: 0.949, y: 0.449, radius: 0.439, smoothing: 1 },
      { x: 0.998, y: 0.478, radius: 0.174 }
    ],
    reps: 16,
    mirror: true
  },
  flower: {
    points: [
      { x: 0.37, y: 0.187 },
      { x: 0.416, y: 0.049, radius: 0.381 },
      { x: 0.479, y: 0.001, radius: 0.095 }
    ],
    reps: 8,
    mirror: true
  },
  puffy: { points: puffyPoints, reps: 2, mirror: true, scaleY: 0.742 },
  puffyDiamond: {
    points: [
      { x: 0.87, y: 0.13, radius: 0.146 },
      { x: 0.818, y: 0.357 },
      { x: 1, y: 0.332, radius: 0.853 }
    ],
    reps: 4,
    mirror: true
  },
  ghostish: {
    points: [
      { x: 0.5, y: 0, radius: 1 },
      { x: 1, y: 0, radius: 1 },
      { x: 1, y: 1.14, radius: 0.254, smoothing: 0.106 },
      { x: 0.575, y: 0.906, radius: 0.253 }
    ],
    mirror: true
  },
  pixelCircle: {
    points: [
      { x: 0.5, y: 0 },
      { x: 0.704, y: 0 },
      { x: 0.704, y: 0.065 },
      { x: 0.843, y: 0.065 },
      { x: 0.843, y: 0.148 },
      { x: 0.926, y: 0.148 },
      { x: 0.926, y: 0.296 },
      { x: 1, y: 0.296 }
    ],
    reps: 2,
    mirror: true
  },
  pixelTriangle: {
    points: [
      { x: 0.11, y: 0.5 },
      { x: 0.113, y: 0 },
      { x: 0.287, y: 0 },
      { x: 0.287, y: 0.087 },
      { x: 0.421, y: 0.087 },
      { x: 0.421, y: 0.17 },
      { x: 0.56, y: 0.17 },
      { x: 0.56, y: 0.265 },
      { x: 0.674, y: 0.265 },
      { x: 0.675, y: 0.344 },
      { x: 0.789, y: 0.344 },
      { x: 0.789, y: 0.439 },
      { x: 0.888, y: 0.439 }
    ],
    mirror: true
  },
  bun: {
    points: [
      { x: 0.796, y: 0.5 },
      { x: 0.853, y: 0.518, radius: 1 },
      { x: 0.992, y: 0.631, radius: 1 },
      { x: 0.968, y: 1, radius: 1 }
    ],
    reps: 2,
    mirror: true
  },
  heart: {
    points: [
      { x: 0.5, y: 0.268, radius: 0.016 },
      { x: 0.792, y: -0.066, radius: 0.958 },
      { x: 1.064, y: 0.276, radius: 1 },
      { x: 0.501, y: 0.946, radius: 0.129 }
    ],
    mirror: true
  }
} satisfies Record<string, ShapeDef>;

export type MaterialShapeName = keyof typeof SHAPE_DEFS;

const MATERIAL_SHAPE_NAMES = Object.keys(SHAPE_DEFS) as MaterialShapeName[];

function repeatPoints(points: ShapeVertex[], reps: number, mirror: boolean): ShapeVertex[] {
  const result: ShapeVertex[] = [];
  if (mirror) {
    const measures = points.map((point) => {
      const dx = point.x - CENTER.x;
      const dy = point.y - CENTER.y;
      return { angle: Math.atan2(dy, dx), distance: Math.hypot(dx, dy) };
    });
    const actualReps = reps * 2;
    const sectionAngle = (Math.PI * 2) / actualReps;
    for (let r = 0; r < actualReps; r += 1) {
      for (let index = 0; index < points.length; index += 1) {
        const i = r % 2 === 0 ? index : points.length - 1 - index;
        if (i > 0 || r % 2 === 0) {
          const angle =
            sectionAngle * r +
            (r % 2 === 0
              ? measures[i].angle
              : sectionAngle - measures[i].angle + 2 * measures[0].angle);
          result.push({
            x: Math.cos(angle) * measures[i].distance + CENTER.x,
            y: Math.sin(angle) * measures[i].distance + CENTER.y,
            radius: points[i].radius,
            smoothing: points[i].smoothing
          });
        }
      }
    }
    return result;
  }

  const count = points.length;
  for (let i = 0; i < count * reps; i += 1) {
    const point = points[i % count];
    const angle = (Math.floor(i / count) * Math.PI * 2) / reps;
    const dx = point.x - CENTER.x;
    const dy = point.y - CENTER.y;
    result.push({
      x: Math.cos(angle) * dx - Math.sin(angle) * dy + CENTER.x,
      y: Math.sin(angle) * dx + Math.cos(angle) * dy + CENTER.y,
      radius: point.radius,
      smoothing: point.smoothing
    });
  }
  return result;
}

interface Corner {
  start: Point;
  control: Point;
  end: Point;
}

function buildCorners(vertices: ShapeVertex[]): Corner[] {
  const count = vertices.length;
  const roundCut = new Array<number>(count).fill(0);
  const smoothCut = new Array<number>(count).fill(0);
  const directions = new Array<{ back: Point; forward: Point } | null>(count).fill(null);

  for (let index = 0; index < count; index += 1) {
    const current = vertices[index];
    const previous = vertices[(index + count - 1) % count];
    const next = vertices[(index + 1) % count];
    const backX = previous.x - current.x;
    const backY = previous.y - current.y;
    const forwardX = next.x - current.x;
    const forwardY = next.y - current.y;
    const backLength = Math.hypot(backX, backY);
    const forwardLength = Math.hypot(forwardX, forwardY);
    if (backLength < 1e-6 || forwardLength < 1e-6) continue;

    const back = { x: backX / backLength, y: backY / backLength };
    const forward = { x: forwardX / forwardLength, y: forwardY / forwardLength };
    const cosine = back.x * forward.x + back.y * forward.y;
    const sine = Math.sqrt(Math.max(0, 1 - cosine * cosine));
    const radius = current.radius ?? 0;
    const expected = sine > 1e-3 ? (radius * (cosine + 1)) / sine : 0;
    roundCut[index] = expected;
    smoothCut[index] = (1 + (current.smoothing ?? 0)) * expected;
    directions[index] = { back, forward };
  }

  const roundRatio = new Array<number>(count).fill(1);
  const smoothRatio = new Array<number>(count).fill(1);
  for (let index = 0; index < count; index += 1) {
    const nextIndex = (index + 1) % count;
    const current = vertices[index];
    const next = vertices[nextIndex];
    const sideLength = Math.hypot(next.x - current.x, next.y - current.y);
    const neededRound = roundCut[index] + roundCut[nextIndex];
    const neededCut = smoothCut[index] + smoothCut[nextIndex];
    if (neededRound > sideLength && neededRound > 0) {
      roundRatio[index] = sideLength / neededRound;
      smoothRatio[index] = 0;
    } else if (neededCut > sideLength) {
      roundRatio[index] = 1;
      smoothRatio[index] =
        (sideLength - neededRound) / (neededCut - neededRound || 1);
    }
  }

  const corners: Corner[] = [];
  for (let index = 0; index < count; index += 1) {
    const current = vertices[index];
    const direction = directions[index];
    if (!direction) {
      corners.push({
        start: { x: current.x, y: current.y },
        control: { x: current.x, y: current.y },
        end: { x: current.x, y: current.y }
      });
      continue;
    }
    const previousSide = (index + count - 1) % count;
    const startCut =
      roundCut[index] * roundRatio[previousSide] +
      (smoothCut[index] - roundCut[index]) * smoothRatio[previousSide];
    const endCut =
      roundCut[index] * roundRatio[index] +
      (smoothCut[index] - roundCut[index]) * smoothRatio[index];
    corners.push({
      start: {
        x: current.x + direction.back.x * startCut,
        y: current.y + direction.back.y * startCut
      },
      control: { x: current.x, y: current.y },
      end: {
        x: current.x + direction.forward.x * endCut,
        y: current.y + direction.forward.y * endCut
      }
    });
  }
  return corners;
}

function flattenCorners(corners: Corner[]): Point[] {
  const points: Point[] = [];
  const push = (x: number, y: number) => points.push({ x, y });

  const first = corners[0].start;
  push(first.x, first.y);

  for (let index = 0; index < corners.length; index += 1) {
    const corner = corners[index];
    for (let step = 1; step <= 8; step += 1) {
      const t = step / 8;
      const inverse = 1 - t;
      push(
        inverse * inverse * corner.start.x + 2 * inverse * t * corner.control.x + t * t * corner.end.x,
        inverse * inverse * corner.start.y + 2 * inverse * t * corner.control.y + t * t * corner.end.y
      );
    }

    const next = corners[(index + 1) % corners.length];
    const distance = Math.hypot(next.start.x - corner.end.x, next.start.y - corner.end.y);
    const steps = Math.max(2, Math.ceil(distance * 48));
    for (let step = 1; step <= steps; step += 1) {
      const t = step / steps;
      push(
        corner.end.x + (next.start.x - corner.end.x) * t,
        corner.end.y + (next.start.y - corner.end.y) * t
      );
    }
  }

  return points;
}

function normalizePoints(points: Point[]): Point[] {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const point of points) {
    minX = Math.min(minX, point.x);
    minY = Math.min(minY, point.y);
    maxX = Math.max(maxX, point.x);
    maxY = Math.max(maxY, point.y);
  }
  const width = maxX - minX;
  const height = maxY - minY;
  const side = Math.max(width, height);
  if (!Number.isFinite(side) || side < 1e-6) {
    return [{ x: SIZE / 2, y: SIZE / 2 }];
  }
  const scale = SIZE / side;
  const offsetX = (SIZE - width * scale) / 2;
  const offsetY = (SIZE - height * scale) / 2;
  return points.map((point) => ({
    x: (point.x - minX) * scale + offsetX,
    y: (point.y - minY) * scale + offsetY
  }));
}

function areaCentroid(points: Point[]): Point {
  let area = 0;
  let centroidX = 0;
  let centroidY = 0;
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const next = points[(index + 1) % points.length];
    const cross = current.x * next.y - next.x * current.y;
    area += cross;
    centroidX += (current.x + next.x) * cross;
    centroidY += (current.y + next.y) * cross;
  }
  area *= 0.5;
  if (Math.abs(area) < 1e-6) return { x: SIZE / 2, y: SIZE / 2 };
  return { x: centroidX / (6 * area), y: centroidY / (6 * area) };
}

function centerByCentroid(points: Point[]): Point[] {
  const centroid = areaCentroid(points);
  const shifted = points.map((point) => ({
    x: point.x - centroid.x,
    y: point.y - centroid.y
  }));
  let extent = 0;
  for (const point of shifted) {
    extent = Math.max(extent, Math.abs(point.x), Math.abs(point.y));
  }
  const scale = extent > 0 ? SIZE / 2 / extent : 1;
  return shifted.map((point) => ({
    x: SIZE / 2 + point.x * scale,
    y: SIZE / 2 + point.y * scale
  }));
}

function signedArea(points: Point[]): number {
  let area = 0;
  for (let index = 0; index < points.length; index += 1) {
    const current = points[index];
    const next = points[(index + 1) % points.length];
    area += current.x * next.y - next.x * current.y;
  }
  return area / 2;
}

function resample(points: Point[], count: number): Float32Array {
  const cleaned: Point[] = [];
  for (const point of points) {
    const previous = cleaned[cleaned.length - 1];
    if (!previous || Math.hypot(point.x - previous.x, point.y - previous.y) > 1e-9) {
      cleaned.push(point);
    }
  }
  if (cleaned.length > 1) {
    const first = cleaned[0];
    const last = cleaned[cleaned.length - 1];
    if (Math.hypot(first.x - last.x, first.y - last.y) <= 1e-9) cleaned.pop();
  }
  if (cleaned.length < 3) {
    return new Float32Array(count * 2);
  }

  const lengths: number[] = [];
  let total = 0;
  for (let index = 0; index < cleaned.length; index += 1) {
    const current = cleaned[index];
    const next = cleaned[(index + 1) % cleaned.length];
    const length = Math.hypot(next.x - current.x, next.y - current.y);
    lengths.push(length);
    total += length;
  }
  if (total < 1e-9) {
    return new Float32Array(count * 2);
  }

  let startIndex = 0;
  let bestY = Infinity;
  let bestX = Infinity;
  for (let index = 0; index < cleaned.length; index += 1) {
    const point = cleaned[index];
    if (point.y < bestY - 1e-6 || (Math.abs(point.y - bestY) <= 1e-6 && point.x < bestX)) {
      bestY = point.y;
      bestX = point.x;
      startIndex = index;
    }
  }

  const result = new Float32Array(count * 2);
  const step = total / count;
  let segment = startIndex;
  let remaining = lengths[segment];
  let distance = 0;

  for (let index = 0; index < count; index += 1) {
    let guard = 0;
    while (guard < cleaned.length && (remaining <= 0 || distance > remaining)) {
      distance -= Math.max(0, remaining);
      segment = (segment + 1) % cleaned.length;
      remaining = lengths[segment];
      guard += 1;
    }
    const current = cleaned[segment];
    const next = cleaned[(segment + 1) % cleaned.length];
    const t = remaining > 0 ? Math.min(1, distance / remaining) : 0;
    result[index * 2] = current.x + (next.x - current.x) * t;
    result[index * 2 + 1] = current.y + (next.y - current.y) * t;
    distance += step;
  }

  return result;
}

const pointsCache = new Map<MaterialShapeName, Float32Array>();
const pathCache = new Map<MaterialShapeName, string>();

export function shapePoints(name: MaterialShapeName): Float32Array {
  const cached = pointsCache.get(name);
  if (cached) return cached;

  const definition: ShapeDef = SHAPE_DEFS[name];
  let vertices = repeatPoints(
    definition.points,
    definition.reps ?? 1,
    definition.mirror ?? false
  );
  if (definition.scaleY && definition.scaleY !== 1) {
    vertices = vertices.map((vertex) => ({
      ...vertex,
      y: CENTER.y + (vertex.y - CENTER.y) * definition.scaleY!
    }));
  }

  let points = normalizePoints(flattenCorners(buildCorners(vertices)));
  if (points.length < 3) points = flattenCorners(buildCorners(regularVertices(32, 0.5)));
  points = centerByCentroid(points);
  if (definition.scale && definition.scale !== 1) {
    points = points.map((point) => ({
      x: SIZE / 2 + (point.x - SIZE / 2) * definition.scale!,
      y: SIZE / 2 + (point.y - SIZE / 2) * definition.scale!
    }));
  }
  if (signedArea(points) < 0) points.reverse();

  const sampled = resample(points, SHAPE_SAMPLES);
  pointsCache.set(name, sampled);
  return sampled;
}

export function pointsToPath(points: ArrayLike<number>): string {
  const count = points.length / 2;
  const format = (value: number) => String(Math.round(value * 100) / 100);
  let path = `M ${format(points[0])} ${format(points[1])}`;
  for (let index = 0; index < count; index += 1) {
    const previousX = points[((index - 1 + count) % count) * 2];
    const previousY = points[((index - 1 + count) % count) * 2 + 1];
    const currentX = points[index * 2];
    const currentY = points[index * 2 + 1];
    const nextX = points[((index + 1) % count) * 2];
    const nextY = points[((index + 1) % count) * 2 + 1];
    const afterX = points[((index + 2) % count) * 2];
    const afterY = points[((index + 2) % count) * 2 + 1];
    const firstX = currentX + (nextX - previousX) / 6;
    const firstY = currentY + (nextY - previousY) / 6;
    const secondX = nextX - (afterX - currentX) / 6;
    const secondY = nextY - (afterY - currentY) / 6;
    path += ` C ${format(firstX)} ${format(firstY)} ${format(secondX)} ${format(secondY)} ${format(nextX)} ${format(nextY)}`;
  }
  return `${path} Z`;
}

export function materialShapePath(name: MaterialShapeName): string {
  const cached = pathCache.get(name);
  if (cached) return cached;
  const path = pointsToPath(shapePoints(name));
  pathCache.set(name, path);
  return path;
}

export function interpolatePoints(
  from: ArrayLike<number>,
  to: ArrayLike<number>,
  t: number
): Float32Array {
  const points = new Float32Array(SHAPE_SAMPLES * 2);
  for (let index = 0; index < points.length; index += 1) {
    points[index] = from[index] + (to[index] - from[index]) * t;
  }
  return points;
}

function hashSeed(seed: string): number {
  let hash = 2166136261;
  for (let index = 0; index < seed.length; index += 1) {
    hash ^= seed.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

const ACTIVE_EXCLUDED = new Set<MaterialShapeName>(['circle', 'arrow', 'semiCircle', 'triangle']);
const ACTIVE_SHAPES = MATERIAL_SHAPE_NAMES.filter((name) => !ACTIVE_EXCLUDED.has(name));

const DANGER_SHAPE_CYCLE: MaterialShapeName[] = [
  'burst',
  'boom',
  'softBurst',
  'verySunny',
  'sunny',
  'cookie12Sided',
  'cookie9Sided',
  'cookie7Sided',
  'puffyDiamond',
  'clover8Leaf',
  'fan'
];

function greatestCommonDivisor(a: number, b: number): number {
  let x = a;
  let y = b;
  while (y !== 0) {
    [x, y] = [y, x % y];
  }
  return x;
}

// Spread each open over the pool: a fixed per-open seed keeps the order stable while the dialog is visible.
export function heroShapeCycle(seed: string, tone: 'accent' | 'danger'): MaterialShapeName[] {
  const pool = tone === 'danger' ? DANGER_SHAPE_CYCLE : ACTIVE_SHAPES;
  const start = hashSeed(`${seed}\u0001${tone}\u0001start`) % pool.length;
  let step = 1 + (hashSeed(`${seed}\u0001${tone}\u0001step`) % (pool.length - 1));
  while (greatestCommonDivisor(step, pool.length) !== 1) step += 1;
  return Array.from({ length: pool.length }, (_, index) => pool[(start + index * step) % pool.length]);
}

const SEMANTIC_ACTIVE_SHAPES: Partial<Record<string, MaterialShapeName>> = {
  feed: 'square',
  creators: 'clover4Leaf',
  favorites: 'heart',
  library: 'slanted',
  downloads: 'softBoom',
  settings: 'sunny',
  profile: 'oval',
  appearance: 'flower',
  interface: 'pill',
  grid: 'square',
  providers: 'sunny',
  notifications: 'burst',
  network: 'diamond',
  sync: 'bun',
  cache: 'arch',
  about: 'heart'
};

export function activeMaterialShapeFor(seed: string): MaterialShapeName {
  const semantic = SEMANTIC_ACTIVE_SHAPES[seed];
  if (semantic) return semantic;
  return ACTIVE_SHAPES[hashSeed(`${seed}\u0001active`) % ACTIVE_SHAPES.length];
}
