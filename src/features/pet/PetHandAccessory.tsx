import { useId } from "react";
import type { PetFrame } from "./types";

// Traced in the source SVG's native units. The raised paws share a shape,
// but the striking paws have their own silhouette and wrist direction.
const raised = {
  outline: "M32 90 Q26 81 23 68 Q19 56 30 54 Q40 51 56 67 L61 74Z",
  wrist: [44, 78, -30, 34],
} as const;
const strikingLeft = {
  outline: "M35 77 L63 91 L45 99 Q31 105 28 97 Q25 91 35 77Z",
  wrist: [46, 87, 27, 33],
} as const;
const strikingRight = {
  outline: "M137 78 L157 94 L135 105 Q127 109 125 102 Q124 95 137 78Z",
  wrist: [143, 90, 39, 28],
} as const;
const paws = {
  up: [{ shape: raised, x: 0, y: 0 }, { shape: raised, x: 101, y: 1 }],
  left: [{ shape: strikingLeft, x: 0, y: 0 }, { shape: raised, x: 97, y: -8 }],
  right: [{ shape: raised, x: -6, y: -6 }, { shape: strikingRight, x: 0, y: 0 }],
} as const;

export function PetHandAccessory({ id, frame }: {
  id: "assassin-wraps" | "smith-mitts" | "alchemist-cuffs" | "winter-cuffs" | "rhythm-wraps" | "gardener-cuffs" | "sailor-cuffs" | "spark-cuffs";
  frame: PetFrame;
}) {
  const instance = useId().replace(/:/g, "");
  const mitts = id === "smith-mitts";
  const wraps = id === "assassin-wraps";
  const winter = id === "winter-cuffs";
  const rhythm = id === "rhythm-wraps";
  const gardener = id === "gardener-cuffs";
  const sailor = id === "sailor-cuffs";
  const spark = id === "spark-cuffs";
  const color = mitts ? "#a47d5d" : wraps ? "#817490" : winter ? "#8aadc0" : rhythm ? "#bd7b88" : gardener ? "#90a780" : sailor ? "#739ab4" : spark ? "#9586b2" : "#78998f";
  const trim = mitts ? "#dfc095" : wraps ? "#d9c8dd" : winter ? "#fff2dc" : rhythm ? "#ecd69c" : gardener ? "#dfdeb4" : sailor ? "#eee0bd" : spark ? "#ecd5a0" : "#c3d1af";
  return <g>{paws[frame].map(({ shape, x, y }, index) => {
    const clip = `${instance}-paw-${index}`;
    const [cx, cy, angle, width] = shape.wrist;
    return <g key={index} transform={`translate(${x} ${y})`}>
      <defs><clipPath id={clip}><path d={shape.outline} /></clipPath></defs>
      {mitts && <path d={shape.outline} fill={color} strokeWidth="2" />}
      <g clipPath={`url(#${clip})`}>
        <g transform={`translate(${cx} ${cy}) rotate(${angle})`}>
          <path d={`M${-width / 2 - 2} -5 Q0 -2 ${width / 2 + 2} -5 L${width / 2 + 2} 5 Q0 9 ${-width / 2 - 2} 5Z`}
            fill={color} strokeWidth="1.6" />
          <path d={`M${-width / 2} -2 Q0 1 ${width / 2} -2 M${-width / 2} 3 Q0 6 ${width / 2} 3`}
            fill="none" stroke={trim} strokeWidth="1.5" />
          {winter ? <path d={`M${-width / 2} -2 Q0 1 ${width / 2} -2`} fill="none" stroke={trim} strokeWidth="5" strokeDasharray="1 2" />
            : rhythm ? <g fill={trim} stroke={trim} strokeWidth="1.5"><path d="M1 3 V-3 L5 -4" fill="none" /><ellipse cx="-1" cy="4" rx="2.5" ry="1.5" /></g>
            : gardener ? <path d="M0 5 V-3 M0 1 Q-8 1 -6 -4 Q-1 -4 0 1 M0 0 Q6 -5 7 -1 Q5 3 0 0" fill={trim} stroke={trim} strokeWidth="1" />
            : sailor ? <g fill="none" stroke={trim} strokeWidth="1.5"><ellipse cx="-2" cy="1" rx="4" ry="2" /><ellipse cx="3" cy="1" rx="4" ry="2" /><path d="M-6 1 H-10 M7 1 H11" /></g>
            : spark ? <path d="M0 -5 L2 -1 L6 1 L2 3 L0 7 L-2 3 L-6 1 L-2 -1Z" fill={trim} strokeWidth="1" />
            : wraps ? <path d="M-9 -4 L-3 6 M0 -3 L6 6 M9 -4 L14 5" fill="none" stroke={trim} strokeWidth="1.5" />
            : mitts ? <rect x="-4" y="-3" width="8" height="7" rx="1.5" fill="#deb771" strokeWidth="1.5" />
              : <path d="M0 -4 L4 1 L0 6 L-4 1Z" fill="#bcd7df" strokeWidth="1.5" />}
        </g>
      </g>
    </g>;
  })}</g>;
}
