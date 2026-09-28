import {spawnSync} from 'node:child_process';
import {mkdirSync,copyFileSync} from 'node:fs';
const development=process.argv.includes('--dev');
function run(command,args){const result=spawnSync(command,args,{stdio:'inherit',shell:process.platform==='win32'});if(result.status!==0)process.exit(result.status||1)}
run('cargo',['build','--locked','-p','yogo-pet-hook',...development?[]:['--release']]);
const binary=process.platform==='win32'?'yogo-pet-hook.exe':'yogo-pet-hook';
mkdirSync('src-tauri/resources',{recursive:true});
copyFileSync(`target/${development?'debug':'release'}/${binary}`,`src-tauri/resources/${binary}`);
run('npx',['tauri',development?'dev':'build',...development?[]:['--bundles','app']]);
