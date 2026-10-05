import {spawnSync} from 'node:child_process';
import {mkdirSync,copyFileSync} from 'node:fs';
const development=process.argv.includes('--dev');
function run(command,args){const result=spawnSync(command,args,{stdio:'inherit',shell:process.platform==='win32'});if(result.status!==0)process.exit(result.status||1)}
run('cargo',['build','--locked','-p','yogosync-hook',...development?[]:['--release']]);
const binary=process.platform==='win32'?'yogosync-hook.exe':'yogosync-hook';
mkdirSync('src-tauri/resources',{recursive:true});
copyFileSync(`target/${development?'debug':'release'}/${binary}`,`src-tauri/resources/${binary}`);
run('npx',['tauri',development?'dev':'build',...development?[]:['--bundles','app']]);
