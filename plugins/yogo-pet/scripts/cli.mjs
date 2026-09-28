import { YogoDevice, listDevices } from './device.mjs';
import { frameFor } from './art.mjs';
import { mkdir, writeFile, readFile } from 'node:fs/promises';
const command = process.argv[2];
if (command === 'devices') console.log(JSON.stringify(await listDevices(), null, 2));
else if (command === 'inspect' || command === 'demo' || command === 'restore') {
  const device = await YogoDevice.open();
  try {
    if (command === 'inspect') console.log(JSON.stringify({ model: device.info.product, block: [...await device.readDotBlock()] }));
    if (command === 'demo') {
      await device.begin(async backup => { await mkdir('.runtime', {recursive:true}); await writeFile('.runtime/device-backup.json', JSON.stringify(backup)); });
      for (const state of ['thinking','working','done']) {
        await device.writeFrame(frameFor(state)); console.log(state);
        await new Promise(r => setTimeout(r, 2500));
      }
      await device.restore(); console.log('original effect restored');
    }
    if (command === 'restore') {
      const backup = JSON.parse(await readFile('.runtime/device-backup.json','utf8'));
      if (backup.serialNumber !== device.info.serialNumber) throw new Error('Backup belongs to a different device');
      await device.restore(backup.block); console.log('restored');
    }
  } finally { if (device.original) await device.restore().catch(e=>console.error(e.message)); await device.close(); }
} else throw new Error('Usage: devices | inspect | demo | restore');
