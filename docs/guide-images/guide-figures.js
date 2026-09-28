// Captured from the real React components with browser-only sample data.
// Preserve the source PNGs; crop, emphasis and labels are rendered as SVG/HTML.
const GUIDE_FIGURES = {
  "recognition": {
    "title": "先选监听账号，再准备识别 Mod",
    "file": "recognition.png",
    "crop": [
      355,
      68,
      773,
      375
    ],
    "note": "图中尚未准备识别 Mod。开关打开不等于已经开始捕获；完成准备并重启游戏后，请以运行状态为准。",
    "marks": [
      [
        958,
        267,
        151,
        39,
        "选择实际刷图的账号"
      ],
      [
        376,
        339,
        733,
        40,
        "选择账号使用的 Mod；缺少声纹功能时前往加工"
      ],
      [
        1059,
        94,
        50,
        32,
        "Mod 准备并重启游戏后，再开启自动识别"
      ],
      [
        376,
        395,
        733,
        31,
        "展开诊断工具查看捕获状态与地点信号"
      ]
    ]
  },
  "mod": {
    "title": "Mod 加工：先准备加工器，再设置目标与功能",
    "file": "mod-processing.png",
    "crop": [
      355,
      111,
      773,
      588
    ],
    "note": "此图以原版游戏为来源，选择声纹识别与局内房间工具，输出名为 HubMain。加工器尚未安装，因此开始按钮不可用；示例没有执行加工。",
    "marks": [
      [
        1005,
        124,
        106,
        39,
        "加工器缺失时，先打开“下载与更新”安装"
      ],
      [
        376,
        300,
        282,
        36,
        "选择加工完成后要应用的目标账号"
      ],
      [
        376,
        590,
        282,
        68,
        "选择原版游戏或已有 Mod 作为来源"
      ],
      [
        691,
        253,
        418,
        113,
        "按需选择声纹识别与局内房间工具"
      ],
      [
        691,
        576,
        418,
        106,
        "填写新名称，准备完成后开始加工并应用"
      ]
    ]
  },
  "participants": {
    "title": "跟房准备：主号、小号、Mod 都要对应",
    "file": "room-participants.png",
    "crop": [
      355,
      196,
      773,
      509
    ],
    "note": "图中主账号的 Mod 尚缺少局内房间工具，小号为已就绪示例。向下滚动，逐个确认所有参与账号；不要只检查主号。",
    "marks": [
      [
        378,
        305,
        273,
        43,
        "指定唯一的主账号"
      ],
      [
        379,
        426,
        216,
        42,
        "勾选需要跟随的小号"
      ],
      [
        490,
        562,
        164,
        133,
        "逐个确认参与账号的 Mod 含有局内房间工具"
      ]
    ]
  },
  "method": {
    "title": "先手动跟随，再选择小号进房方式",
    "file": "room-participants.png",
    "crop": [
      711,
      203,
      408,
      353
    ],
    "note": "当前使用后台同步预填，创建房间时保持主号在前台。首次先用手动跟随验证，再按需要切换自动跟随。",
    "marks": [
      [
        724,
        376,
        177,
        44,
        "首次使用先选手动跟随"
      ],
      [
        918,
        376,
        177,
        44,
        "选择同时或间隔派发小号"
      ],
      [
        724,
        479,
        116,
        65,
        "设置主账号创建房间快捷键"
      ],
      [
        918,
        479,
        116,
        65,
        "设置跟随账号加入快捷键"
      ]
    ]
  },
  "paths": {
    "title": "只配置你实际使用的客户端",
    "file": "paths.png",
    "crop": [
      408,
      60,
      717,
      641
    ],
    "note": "路径均为示例，不要照抄。国服与国际服分开选择；存档目录服务于账号独立画质配置，缺少 Settings.json 时按页面提示处理。",
    "marks": [
      [
        424,
        266,
        326,
        128,
        "国服玩家：选择国服游戏与存档位置"
      ],
      [
        424,
        565,
        326,
        129,
        "国际服玩家：使用国际服目录"
      ],
      [
        783,
        254,
        326,
        113,
        "网页 Token 流程使用隔离浏览器"
      ]
    ]
  },
  "init": {
    "title": "添加账号：从本地昵称开始",
    "file": "account-init.png",
    "crop": [
      831,
      238,
      433,
      245
    ],
    "note": "这里只填写用于辨认账号的昵称，不是战网密码。后续按向导选择区服、认证方式并完成登录。",
    "marks": [
      [
        850,
        361,
        394,
        46,
        "先填容易区分的昵称，例如主号或小号1"
      ],
      [
        850,
        416,
        394,
        36,
        "点击下一步，按顶部顺序继续初始化"
      ]
    ]
  },
  "accounts": {
    "title": "多开账号各自保存配置",
    "file": "accounts.png",
    "crop": [
      403,
      54,
      723,
      408
    ],
    "note": "这是设置中心的账号配置页，不是游戏启动按钮。完成初始化后，请回主界面账号卡片启动；此图账号与状态均为示例。",
    "marks": [
      [
        415,
        68,
        246,
        281,
        "先选择需要配置的账号"
      ],
      [
        903,
        189,
        209,
        66,
        "确认当前账号实际选用的 Mod"
      ],
      [
        416,
        355,
        244,
        91,
        "未初始化的账号要先完成认证"
      ]
    ]
  },
  "modules": {
    "title": "按需添加扩展，从同一侧栏进入设置",
    "file": "modules.png",
    "crop": [
      143,
      64,
      991,
      541
    ],
    "note": "图中扩展均已添加，因此显示“设置”。首次使用先点“添加”；添加后可从左侧直接进入。识别与跟房还需要对应游戏 Mod，移除扩展会停止相关功能并保留配置。",
    "marks": [
      [
        1000,
        278,
        83,
        40,
        "识别与统计：添加后进入设置"
      ],
      [
        1000,
        379,
        83,
        40,
        "自动跟房：添加后进入设置"
      ],
      [
        149,
        304,
        176,
        142,
        "已添加扩展也可直接从左侧进入"
      ]
    ]
  },
  "rules": {
    "title": "房名由前缀、序号和位数一起决定",
    "file": "room-rules.png",
    "crop": [
      711,
      372,
      408,
      218
    ],
    "note": "示例 chaos- + 27 + 3 位得到 chaos-027。密码 pw 仅为示例，留空则创建无密码房间；修改会自动保存。",
    "marks": [
      [
        724,
        436,
        383,
        66,
        "填写房名开头、下一个序号和序号位数"
      ],
      [
        1017,
        390,
        88,
        23,
        "核对下次房名预览"
      ],
      [
        724,
        506,
        153,
        59,
        "密码可选；留空就是无密码房间"
      ]
    ]
  },
  "auto": {
    "title": "切换自动跟随，并留足主号载入时间",
    "file": "room-auto.png",
    "crop": [
      711,
      201,
      408,
      433
    ],
    "note": "5 秒只是示例，应覆盖你机器上主号的实际载入时间。自动跟随仍由你触发主号建房；本图只展示配置，没有执行跟房任务。",
    "marks": [
      [
        810,
        375,
        93,
        44,
        "切到自动跟随"
      ],
      [
        724,
        480,
        120,
        57,
        "设置建房后等待秒数"
      ],
      [
        918,
        375,
        177,
        44,
        "选择小号进房方式"
      ],
      [
        918,
        480,
        116,
        62,
        "使用建房快捷键触发新房"
      ]
    ]
  }
};
const GUIDE_STEP_FIGURES = {
  paths: ['paths'], first: ['init'], second: ['init', 'accounts'],
  'audio-install': ['modules', 'recognition'], 'audio-mod': ['mod'], 'combined-mod': ['mod', 'accounts'],
  'room-install': ['modules', 'participants'], 'room-mod': ['participants', 'mod'],
  'room-method': ['method'], 'room-rules': ['rules', 'method'], 'room-auto': ['auto']
};
function guideStepFigureMarkup(stepId) {
  const keys = GUIDE_STEP_FIGURES[stepId];
  if (!keys) return '';
  const picker = keys.length > 1 ? `<div class="guide-gallery-picker" aria-label="本步图示顺序">${keys.map((key,index)=>`<button type="button" data-gallery-key="${key}" aria-pressed="${index === 0}">${index + 1}. ${GUIDE_FIGURES[key].title}</button>`).join('')}</div>` : '';
  return `<div class="guide-gallery">${picker}<div class="guide-gallery-content">${guideFigureMarkup(keys[0])}</div></div>`;
}
function bindGuideGalleries(container) {
  bindGuideFigures(container);
  container.querySelectorAll('.guide-gallery').forEach(gallery => {
    gallery.querySelectorAll('[data-gallery-key]').forEach(button => button.onclick = () => {
      gallery.querySelectorAll('[data-gallery-key]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
      const content = gallery.querySelector('.guide-gallery-content');
      content.innerHTML = guideFigureMarkup(button.dataset.galleryKey);
      bindGuideFigures(content);
    });
  });
}

function guideFigureMarkup(key, base = 'guide-images/') {
  const figure = GUIDE_FIGURES[key];
  if (!figure) return '';
  const [x,y,w,h] = figure.crop;
  return `<figure class="guide-figure" data-figure="${key}">
    <figcaption><strong>${figure.title}</strong><span>真实组件截图 · 示例账号与状态</span></figcaption>
    <svg class="guide-picture" viewBox="${x} ${y} ${w} ${h}" role="img" aria-label="${figure.title}，编号对应下方操作说明">
      <defs><clipPath id="crop-${key}"><rect x="${x}" y="${y}" width="${w}" height="${h}" /></clipPath></defs>
      <g clip-path="url(#crop-${key})">
      <image href="${base}${figure.file}" width="1280" height="720" />
      <path class="guide-shade" fill="rgba(12,18,28,.40)" fill-rule="evenodd" d="" />
      ${figure.marks.map(([mx,my,mw,mh,label],i)=>`<g class="guide-mark" data-mark="${i}"><rect x="${mx}" y="${my}" width="${mw}" height="${mh}" rx="5" /><circle cx="${mx+10}" cy="${my}" r="12" /><text x="${mx+10}" y="${my+1}">${i+1}</text></g>`).join('')}
      </g>
    </svg>
    <div class="guide-image-actions"><span>点编号，单独强调对应位置</span><button type="button" data-highlight="all" aria-pressed="true">显示全部</button><a href="${base}${figure.file}" target="_blank" rel="noopener">打开原图 ↗</a></div>
    <div class="guide-legend">${figure.marks.map((mark,i)=>`<button type="button" data-highlight="${i}" aria-pressed="false"><b>${i+1}</b><span>${mark[4]}</span></button>`).join('')}</div>
    <p class="guide-image-note">${figure.note}</p>
  </figure>`;
}
function bindGuideFigures(container) {
  container.querySelectorAll('[data-figure]').forEach(element => {
    const figure = GUIDE_FIGURES[element.dataset.figure];
    element.querySelectorAll('[data-highlight]').forEach(button => button.onclick = () => {
      const value = button.dataset.highlight;
      element.querySelectorAll('[data-highlight]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
      element.querySelectorAll('[data-mark]').forEach(mark => mark.classList.toggle('is-muted', value !== 'all' && mark.dataset.mark !== value));
      let path = '';
      if (value !== 'all') {
        const [x,y,w,h] = figure.crop, [mx,my,mw,mh] = figure.marks[Number(value)];
        path = `M${x},${y}h${w}v${h}h-${w}z M${mx},${my}h${mw}v${mh}h-${mw}z`;
      }
      element.querySelector('.guide-shade').setAttribute('d', path);
    });
  });
}
