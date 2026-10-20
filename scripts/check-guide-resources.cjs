// Exercise the guide's real route/gallery handlers and check its packaged resources.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { JSDOM } = require('jsdom');

const root = path.resolve(__dirname, '..');
const guide = path.join(root, 'docs/user-guide.html');
const config = JSON.parse(fs.readFileSync(path.join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
const resources = new Set(config.bundle.resources.map(file => path.resolve(root, 'src-tauri', file)));
const dom = new JSDOM(fs.readFileSync(guide, 'utf8'), {
  url: 'https://guide.local/user-guide.html', runScripts: 'outside-only', pretendToBeVisual: true,
});
const { window } = dom;
const packageVersion = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
assert.equal(
  window.document.querySelector('meta[name="d2rhub-version"]')?.content,
  packageVersion,
  'Guide version differs from package.json; update the manual when preparing a release.',
);
const runtimeErrors = [];
window.addEventListener('error', event => runtimeErrors.push(event.error || event.message));
window.HTMLElement.prototype.scrollIntoView = () => {};
const used = new Set([guide]);
function checkResource(relative) {
  assert(!/^(?:[a-z]+:|\/)/i.test(relative), `Unexpected guide resource: ${relative}`);
  const file = path.resolve(path.dirname(guide), relative);
  assert(fs.existsSync(file), `Missing guide resource: ${relative}`);
  assert(resources.has(file), `Guide resource not bundled: ${relative}`);
  used.add(file);
  return file;
}
function imageDimensions(file) {
  const bytes = fs.readFileSync(file);
  if (bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
    return [bytes.readUInt32BE(16), bytes.readUInt32BE(20)];
  }
  // Browser captures may contain JPEG bytes even when the resource ends in .png.
  assert.equal(bytes.readUInt16BE(0), 0xffd8, `Unsupported guide image: ${file}`);
  const frameMarkers = new Set([0xc0, 0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc9, 0xca, 0xcb, 0xcd, 0xce, 0xcf]);
  for (let offset = 2; offset + 7 < bytes.length;) {
    if (bytes[offset++] !== 0xff) continue;
    const marker = bytes[offset++];
    if (marker === 0xff || marker === 0x00) continue;
    if (frameMarkers.has(marker)) return [bytes.readUInt16BE(offset + 5), bytes.readUInt16BE(offset + 3)];
    offset += bytes.readUInt16BE(offset);
  }
  assert.fail(`No dimensions in guide image: ${file}`);
}
try {
  checkResource('user-guide.html');
  for (const link of window.document.querySelectorAll('link[rel="stylesheet"]')) {
    checkResource(link.getAttribute('href'));
  }
  for (const script of window.document.querySelectorAll('script')) {
    window.eval(script.src
      ? fs.readFileSync(checkResource(script.getAttribute('src')), 'utf8')
      : script.textContent);
  }
  let steps = 0;
  function checkImages() {
    for (const image of window.document.querySelectorAll('#stepContent image')) {
      const file = checkResource(image.getAttribute('href'));
      const [width, height] = imageDimensions(file);
      assert.equal(Number(image.getAttribute('width')), width, `Guide image width differs from source: ${file}`);
      assert.equal(Number(image.getAttribute('height')), height, `Guide image height differs from source: ${file}`);
      const [x, y, w, h] = image.closest('svg').getAttribute('viewBox').split(/\s+/).map(Number);
      assert(x >= 0 && y >= 0 && x + w <= width && y + h <= height, `Guide crop exceeds image: ${file}`);
    }
  }
  for (const route of ['multi', 'audio', 'room', 'all']) {
    window.location.hash = `guide-${route}`;
    window.dispatchEvent(new window.HashChangeEvent('hashchange'));
    const count = window.document.querySelectorAll('[data-step]').length;
    assert(count > 0, `Route did not render: ${route}`);
    for (let index = 0; index < count; index++) {
      window.document.querySelector(`[data-step="${index}"]`).click();
      assert(window.document.querySelector('#stepTitle'), `Missing step: ${route}/${index}`);
      checkImages();
      for (const button of window.document.querySelectorAll('[data-gallery-key]')) {
        button.click();
        checkImages();
      }
      steps++;
    }
  }
  const bundledFigures = [...resources].filter(file => file.includes(`${path.sep}guide-images${path.sep}`));
  for (const file of bundledFigures) assert(used.has(file), `Unused bundled guide resource: ${file}`);
  assert.equal(runtimeErrors.length, 0, `Guide runtime errors: ${runtimeErrors.join('; ')}`);
  console.log(`Guide OK: 4 routes, ${steps} steps, ${[...used].filter(file => file.endsWith('.png')).length} referenced screenshots; all resources bundled.`);
} finally {
  window.close();
}
