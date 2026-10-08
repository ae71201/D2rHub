import type { PetFrame } from "./types";

/** Static silhouettes only; input poses move the wings without an animation loop. */
export function PetAtmosphereAccessory({ id, frame }: { id: string; frame: PetFrame }) {
  const body = frame === "left" ? "translate(-4 -9)" : frame === "right" ? "translate(-4 -5)" : undefined;
  const ground = frame === "up" ? undefined : "translate(-4 0)";
  const wing = (side: number) => {
    switch (id) {
      case "feather-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 80 Q45 87 13 69 L21 64 Q4 55 8 46 L19 49 Q3 35 9 24 L23 32 Q9 16 15 12 Q40 27 53 36 Q64 43 66 54 Q64 68 86 80Z" fill="#f2e7cd" />
        <path d="M20 25 Q37 52 73 72 M16 40 Q35 60 62 66 M17 56 Q31 66 47 69" fill="none" stroke="#c7ab76" strokeWidth="1.5" />
      </g>;
      case "bat-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 77 Q67 37 24 16 L8 64 Q24 51 36 76 Q50 60 66 83 Q73 69 86 77Z" fill="#92718d" />
        <path d="M24 17 Q43 32 36 76 M24 17 Q58 31 66 83 M24 17 L9 63" fill="none" stroke="#d1adc2" />
      </g>;
      case "butterfly-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 73 Q48 13 17 16 Q0 35 39 54 Q7 61 24 81 Q56 89 86 73Z" fill="#b0a1ce" />
        <path d="M76 67 Q43 26 23 27 Q17 40 47 49 M72 73 Q45 62 30 67 Q28 79 52 76" fill="none" stroke="#efd2ae" strokeWidth="3" />
        <circle cx="24" cy="35" r="3" fill="#efd2ae" stroke="none" /><circle cx="35" cy="73" r="2" fill="#efd2ae" stroke="none" />
      </g>;
      case "frost-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 77 L15 12 L18 39 L8 31 L19 57 L8 56 L18 74 L44 77 L40 85Z" fill="#b4d3df" />
        <path d="M23 27 L77 72 M22 44 L42 48 L39 31 M22 66 L56 66 L54 46" fill="none" stroke="#f0f5e9" strokeWidth="2" />
      </g>;
      case "clockwork-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 79 L52 68 L30 72 L13 63 L23 57 L9 45 L23 45 L12 26 L27 21 L58 39 L66 56Z" fill="#c8a16d" />
        <path d="M24 32 L78 73 M16 44 L58 58 M21 61 L52 68" fill="none" stroke="#f0d5a2" strokeWidth="3" />
        <circle cx="29" cy="33" r="7" fill="#92775b" /><circle cx="29" cy="33" r="2" fill="#f0d5a2" stroke="none" />
      </g>;
      case "constellation-wings": return <g key={side} transform={side ? "translate(208 0) scale(-1 1)" : undefined}>
        <path d="M86 78 Q47 84 12 64 L19 56 L8 42 L18 38 L9 22 L20 14 Q42 35 58 32 Q66 51 73 56Z" fill="#726994" />
        <path d="M22 28 L38 39 L23 49 L34 63 L57 59 L73 71" fill="none" stroke="#e9d7a3" strokeWidth="1.5" />
        {[[22, 28], [38, 39], [34, 52], [57, 59], [73, 71]].map(([x, y]) => <circle key={x} cx={x} cy={y} r="2.5" fill="#fff0c7" stroke="none" />)}
      </g>;
      default: return null;
    }
  };
  if (id.endsWith("-wings")) return <g transform={body}>{[0, 1].map(wing)}</g>;
  switch (id) {
    case "dawn-aura": return <g transform={ground} stroke="#c9a258">
      <ellipse cx="104" cy="111" rx="79" ry="12" fill="#ebc777" fillOpacity="0.18" />
      <ellipse cx="104" cy="111" rx="68" ry="8" fill="none" stroke="#e3c789" />
      <path d="M23 110 h-7 M185 110 h7 M55 120 l-4 3 M153 120 l4 3 M104 120 v4" fill="none" />
    </g>;
    case "moon-aura": return <g transform={ground} stroke="#9d96c1">
      <ellipse cx="104" cy="111" rx="79" ry="12" fill="#b9b2d7" fillOpacity="0.16" />
      <path d="M32 110 Q104 127 176 110 M42 115 Q104 127 166 115" fill="none" />
      <path d="M23 99 Q13 108 26 113 Q12 116 15 106 Q17 100 23 99Z" fill="#ebd8a0" stroke="none" />
      <path d="M184 101 l2 4 4 2 -4 2 -2 4 -2 -4 -4 -2 4 -2Z" fill="#ebd8a0" strokeWidth="1" />
    </g>;
    case "leaf-aura": return <g transform={ground} stroke="#73997a">
      <ellipse cx="104" cy="111" rx="79" ry="12" fill="#abc39b" fillOpacity="0.16" />
      <path d="M25 112 Q104 132 183 112" fill="none" />
      {[[35, 113, -15], [61, 119, -5], [147, 119, 5], [173, 113, 15]].map(([x, y, angle]) => <path key={x} transform={`translate(${x} ${y}) rotate(${angle})`} d="M-7 0 Q0 -8 7 0 Q0 6 -7 0Z" fill="#a9c290" strokeWidth="1.2" />)}
    </g>;
    case "arcane-aura": return <g transform={ground} stroke="#8d8bb9">
      <ellipse cx="104" cy="111" rx="79" ry="12" fill="#aaa6d6" fillOpacity="0.16" />
      <path d="M27 110 L104 98 L181 110 L104 123Z M42 110 L104 102 L166 110 L104 119Z" fill="none" strokeWidth="1.5" />
      <path d="M19 106 l4 4 -4 4 -4 -4Z M189 106 l4 4 -4 4 -4 -4Z M100 118 l4 4 4 -4" fill="#d7c6e8" strokeWidth="1.2" />
    </g>;
    case "ember-aura": return <g transform={ground} stroke="#c08460">
      <ellipse cx="104" cy="111" rx="79" ry="12" fill="#e2aa75" fillOpacity="0.18" />
      <path d="M29 112 Q104 129 179 112 M42 115 Q104 124 166 115" fill="none" />
      <path d="M18 111 Q12 107 20 98 Q18 106 24 108 Q27 114 18 111Z M182 111 Q177 106 185 99 Q183 106 188 108 Q192 115 182 111Z" fill="#e1ab75" strokeWidth="1.2" />
    </g>;
    case "rainbow-aura": return <g transform={ground} fill="none" strokeWidth="2.5">
      <ellipse cx="104" cy="111" rx="79" ry="12" stroke="#c593a9" />
      <ellipse cx="104" cy="111" rx="71" ry="9" stroke="#dfc18a" />
      <ellipse cx="104" cy="111" rx="63" ry="6" stroke="#91b8ad" />
      <path d="M20 100 v8 M16 104 h8 M188 100 v8 M184 104 h8" stroke="#b5a6cd" strokeWidth="1.5" />
    </g>;
    default: return null;
  }
}
