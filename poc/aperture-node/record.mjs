import {setTimeout as wait} from 'node:timers/promises';
import {stat} from 'node:fs/promises';
import {copyFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {recorder, screens, videoCodecs} from 'aperture';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

async function main() {
  console.log('=== aperture-node POC ===');
  console.log('Node version:', process.version);

  console.log('\n--- Available screens ---');
  const screenList = await screens();
  console.log(JSON.stringify(screenList, null, 2));

  console.log('\n--- Available video codecs ---');
  console.log([...videoCodecs.entries()]);

  const primary = screenList[0];
  if (!primary) {
    throw new Error('No screens detected');
  }
  console.log(`\nPrimary screen: ${primary.name} (id=${primary.id})`);

  const options = {
    fps: 30,
    showCursor: true,
    screenId: primary.id,
    videoCodec: 'h264',
  };
  console.log('\nStarting recording with options:', options);

  const t0 = Date.now();
  await recorder.startRecording(options);
  console.log(`Recording started in ${Date.now() - t0} ms`);

  console.log('Recording for 5 seconds...');
  await wait(5000);

  console.log('Stopping recording...');
  const tmpPath = await recorder.stopRecording();
  console.log('Temp file:', tmpPath);

  const outPath = path.join(__dirname, 'output.mp4');
  await copyFile(tmpPath, outPath);
  const stats = await stat(outPath);
  console.log(`\nOutput saved: ${outPath}`);
  console.log(`File size: ${stats.size} bytes (${(stats.size / 1024 / 1024).toFixed(2)} MB)`);
}

main().catch(err => {
  console.error('ERROR:', err);
  process.exit(1);
});
