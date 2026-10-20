export interface DonutSliceGeometry {
  path: string;
  midAngle: number;
  iconPosition: { x: number; y: number };
  activeTranslate: { x: number; y: number };
}

/**
 * Calculates SVG path and layout for an annular sector slice.
 * Angles measured clockwise from 12 o'clock (-pi/2).
 */
export function calculateDonutSlice(
  index: number,
  totalSlices: number,
  cx: number,
  cy: number,
  rInner: number,
  rOuter: number,
  gapRadians: number = 0.04
): DonutSliceGeometry {
  const span = (2 * Math.PI) / totalSlices;
  const midAngle = -Math.PI / 2 + index * span;
  
  const aStart = midAngle - span / 2 + gapRadians / 2;
  const aEnd = midAngle + span / 2 - gapRadians / 2;

  const x1Outer = cx + rOuter * Math.cos(aStart);
  const y1Outer = cy + rOuter * Math.sin(aStart);

  const x2Outer = cx + rOuter * Math.cos(aEnd);
  const y2Outer = cy + rOuter * Math.sin(aEnd);

  const x1Inner = cx + rInner * Math.cos(aStart);
  const y1Inner = cy + rInner * Math.sin(aStart);

  const x2Inner = cx + rInner * Math.cos(aEnd);
  const y2Inner = cy + rInner * Math.sin(aEnd);

  const largeArc = (aEnd - aStart) > Math.PI ? 1 : 0;

  // Outer arc (sweep 1: clockwise) -> Line to inner -> Inner arc (sweep 0: counter-clockwise) -> Close
  const path = `
    M ${x1Outer.toFixed(2)} ${y1Outer.toFixed(2)}
    A ${rOuter} ${rOuter} 0 ${largeArc} 1 ${x2Outer.toFixed(2)} ${y2Outer.toFixed(2)}
    L ${x2Inner.toFixed(2)} ${y2Inner.toFixed(2)}
    A ${rInner} ${rInner} 0 ${largeArc} 0 ${x1Inner.toFixed(2)} ${y1Inner.toFixed(2)}
    Z
  `.trim().replace(/\s+/g, " ");

  // Icon position at radial midpoint
  const rMid = (rInner + rOuter) / 2;
  const iconX = cx + rMid * Math.cos(midAngle);
  const iconY = cy + rMid * Math.sin(midAngle);

  // Active translation vector outward along normal
  const popDistance = 10;
  const activeTranslate = {
    x: popDistance * Math.cos(midAngle),
    y: popDistance * Math.sin(midAngle),
  };

  return {
    path,
    midAngle,
    iconPosition: { x: iconX, y: iconY },
    activeTranslate,
  };
}
