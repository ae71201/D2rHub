import { PET_FRAME_GEOMETRY } from "./petGeometry";
import type { PetFrame } from "./types";

export function PetEverydayAccessory({ id, frame }: { id: string; frame: PetFrame }) {
  const head = frame === "left" ? "translate(-4 -9)" : frame === "right" ? "translate(-4 -5)" : undefined;
  const pulse = frame === "up" ? "translate(0 -18)" : "translate(0 -20)";
  const [lx, ly, rx, ry] = PET_FRAME_GEOMETRY[frame].eyes;
  switch (id) {
    case "mushroom-cap": return <g transform={head}>
      <path d="M71 44 Q77 15 107 18 Q139 19 148 46 Q111 60 71 44Z" fill="#b97d79" />
      <path d="M73 44 Q109 55 147 46 L140 53 Q109 62 79 52Z" fill="#efdbb8" />
      <ellipse cx="90" cy="31" rx="7" ry="4" fill="#f6e6c9" stroke="none" /><ellipse cx="123" cy="32" rx="8" ry="5" fill="#f6e6c9" stroke="none" />
    </g>;
    case "aviator-cap": return <g transform={head}>
      <path d="M75 48 L77 33 Q103 20 133 34 L141 48 L139 61 L132 61 L130 49 Q103 43 85 51 L83 62 L75 62Z" fill="#ac8868" />
      <path d="M80 36 Q105 29 130 37" fill="none" stroke="#e7cdad" strokeWidth="3" />
      <rect x="85" y="34" width="17" height="12" rx="5" fill="#abc1c4" /><rect x="109" y="35" width="17" height="12" rx="5" fill="#abc1c4" /><path d="M102 39 L109 40" />
    </g>;
    case "lotus-tiara": return <g transform={head}>
      <path d="M75 49 Q108 62 143 49" fill="none" stroke="#9caf8e" strokeWidth="4" />
      <path d="M108 52 Q91 46 92 35 Q105 36 108 52 M108 52 Q125 46 125 35 Q112 36 108 52" fill="#d6afbd" />
      <path d="M108 52 Q96 37 108 24 Q120 37 108 52Z" fill="#efd1d4" /><circle cx="108" cy="52" r="3" fill="#dec180" />
    </g>;
    case "cat-eye-glasses": return <g fill="#d7acb6" fillOpacity="0.2" stroke="#9f7486">
      {[[lx, ly], [rx, ry]].map(([x, y]) => <path key={x} d={`M${x - 13} ${y - 10} Q${x} ${y - 5} ${x + 12} ${y - 8} L${x + 9} ${y + 6} Q${x} ${y + 12} ${x - 10} ${y + 5}Z`} />)}
      <path d={`M${lx + 12} ${ly - 3} Q${(lx + rx) / 2} ${ly - 8} ${rx - 12} ${ry - 3}`} fill="none" />
    </g>;
    case "snowflake-glasses": return <g fill="#c7dee5" fillOpacity="0.15" stroke="#86a9bf">
      {[[lx, ly], [rx, ry]].map(([x, y]) => <g key={x}><path d={`M${x - 11} ${y - 6} L${x} ${y - 12} L${x + 11} ${y - 6} V${y + 6} L${x} ${y + 12} L${x - 11} ${y + 6}Z`} /><path d={`M${x - 5} ${y - 7} l4 -2`} stroke="#dcecf1" /></g>)}
      <path d={`M${lx + 11} ${ly} L${rx - 11} ${ry}`} fill="none" />
    </g>;
    case "star-paint": return <g stroke="#b89162" strokeWidth="1.1">
      {[[lx - 13, ly + 11], [rx + 13, ry + 11]].map(([x, y]) => <g key={x} transform={`translate(${x} ${y})`}><path d="M0 -5 L2 -1 L6 0 L2 2 L0 6 L-2 2 L-6 0 L-2 -1Z" fill="#e6c78c" /><circle cx="-8" cy="4" r="1.5" fill="#c7a4bd" stroke="none" /></g>)}
    </g>;
    case "lavender-scarf": return <g transform={head}>
      <path d="M72 89 Q98 99 125 91 L128 98 Q101 109 70 96Z" fill="#a695bc" /><path d="M110 98 L115 118 L126 116 L121 97Z" fill="#bfadce" />
      <path d="M117 104 L121 113 M115 108 L119 109 M120 105 L118 107" fill="none" stroke="#eff0d4" strokeWidth="1.5" />
    </g>;
    case "moon-pendant": return <g transform={head}>
      <path d="M76 91 Q101 107 124 94" fill="none" stroke="#a798b7" /><path d="M105 95 Q90 103 104 114 Q87 114 90 102 Q94 94 105 95Z" fill="#e5cd95" />
      <circle cx="107" cy="105" r="2" fill="#b5a4c8" stroke="none" />
    </g>;
    case "gear-pendant": return <g transform={head}>
      <path d="M76 91 Q101 107 124 94" fill="none" stroke="#9d825b" />
      <path d="M98 95 H104 L105 99 L109 98 L112 103 L109 106 L110 110 L105 113 L102 110 L98 112 L94 108 L96 104 L93 101 L96 97Z" fill="#cca96e" />
      <circle cx="102" cy="104" r="4" fill="#ece0c6" strokeWidth="1.5" />
    </g>;
    case "rain-cape": case "scholar-cape": case "patchwork-cape": return <g transform={head}>
      <path d="M60 64 Q108 44 151 62 L197 112 Q167 119 148 99 L114 90 L76 95 Q53 115 22 109 L46 76Z" fill={id === "rain-cape" ? "#7aabae" : id === "scholar-cape" ? "#63738f" : "#bf927d"} />
      {id === "rain-cape" ? <><path d="M27 105 Q45 111 60 91 M154 95 Q174 115 192 108" fill="none" stroke="#d2ded1" strokeWidth="3" /><path d="M177 92 Q169 103 177 105 Q185 103 177 92Z" fill="#c8e1df" strokeWidth="1.3" /></>
        : id === "scholar-cape" ? <><path d="M24 108 Q45 114 63 96 M151 94 Q173 116 195 110" fill="none" stroke="#d2b981" strokeWidth="3" /><path d="M168 93 L177 96 L187 93 V106 L178 109 L168 106Z M177 96 V109" fill="#e7dbbf" strokeWidth="1.3" /></>
          : <><path d="M30 100 L46 96 L54 108 L38 111Z" fill="#9eaf98" strokeDasharray="2 2" strokeWidth="1.2" /><path d="M170 94 L185 98 L191 109 L177 112Z" fill="#a698b7" strokeDasharray="2 2" strokeWidth="1.2" /></>}
    </g>;
    case "tiny-lantern": return <g transform={pulse}>
      <path d="M186 63 V59 Q192 51 198 59 V63" fill="none" stroke="#a3885b" /><path d="M180 64 L204 64 L200 70 L183 70Z" fill="#bca071" />
      <path d="M183 70 H200 V91 H183Z" fill="#efd79c" /><path d="M181 91 H202 V95 H181Z" fill="#bca071" /><path d="M186 70 V91 M197 70 V91" stroke="#b2996d" strokeWidth="1.5" />
      <path d="M189 85 Q187 82 192 76 Q196 81 194 85Z" fill="#d7a066" stroke="none" />
    </g>;
    case "yarn-ball": return <g transform={pulse}>
      <circle cx="191" cy="79" r="13" fill="#c6a6b9" /><path d="M183 70 Q200 75 200 84 M178 78 Q188 84 195 91 M180 85 Q190 76 194 67 M182 91 Q199 79 196 68 M202 84 Q209 87 204 96 Q199 101 206 103" fill="none" stroke="#ecd3d7" strokeWidth="1.5" />
    </g>;
    case "seedling-pot": return <g transform={pulse}>
      <path d="M181 79 H203 L200 96 H184Z" fill="#c69478" /><rect x="179" y="76" width="26" height="5" rx="2" fill="#d9b292" />
      <path d="M192 76 V61" stroke="#749473" /><path d="M192 69 Q178 69 179 59 Q191 56 192 69 M192 65 Q193 52 203 54 Q207 64 192 65Z" fill="#a0ba86" stroke="#749473" />
    </g>;
    default: return null;
  }
}
