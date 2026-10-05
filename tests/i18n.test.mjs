import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {create,resolveLocale,catalog} from './i18n-helper.mjs';

test('OS language selects Chinese variants and defaults other locales to English',()=>{
 for(const language of ['zh','zh-CN','zh-TW','zh-Hant-HK','ZH_sg']) assert.equal(resolveLocale(language),'zh-CN');
 for(const language of ['en-US','fr-FR','ja',undefined,'','zho']) assert.equal(resolveLocale(language),'en');
});
test('both locales render states, interpolations, diagnostics and preserve user theme names',()=>{
 const en=create('en-US',catalog),zh=create('zh-TW',catalog);
 assert.equal(en.t('等待确认'),'Awaiting approval');
 assert.equal(zh.t('等待确认'),'等待确认');
 assert.equal(en.t('第 {0} 行，第 {1} 列',2,3),'Row 2, column 3');
 assert.equal(zh.t('第 {0} 行，第 {1} 列',2,3),'第 2 行，第 3 列');
 assert.equal(en.messageText('主题文件格式无效：unexpected token'), 'Invalid theme file: unexpected token');
 assert.equal(en.messageText('设备响应超时 0xAB；请关闭 ATK 设置页后重试'),'Device response timed out (0xAB). Close the ATK settings page and try again');
 assert.equal(en.messageText('服务已关闭；恢复待重试：设备操作超时'),'Service closed; restoration needs retry: Device operation timed out');
 assert.equal(en.themeName('builtin','机器人与符号'),'Robot & Symbols');
 assert.equal(en.themeName('custom','机器人与符号'),'机器人与符号');
 assert.equal(en.themeName('builtin','My edited name'),'My edited name');
 assert.equal(en.messageText('/tmp/用户文件.json'),'/tmp/用户文件.json');
});
test('UI and native translation keys have English entries with matching placeholders',()=>{
 const paths=['desktop/app.js','desktop/themes.js','desktop/settings.js','desktop/glow.js','desktop/codex-connection.js','src-tauri/src/main.rs','src-tauri/src/commands.rs'];
 for(const path of paths){
  for(const match of readFileSync(path,'utf8').matchAll(/\bt\(['"]([^'"\n]+)['"]/g)) assert.ok(catalog[match[1]],`${path}: ${match[1]}`);
 }
 for(const [key,value] of Object.entries(catalog)){
  assert.deepEqual((key.match(/\{[^}]+\}/g)||[]).sort(),(value.match(/\{[^}]+\}/g)||[]).sort(),key);
 }
 for(const path of ['desktop/index.html','desktop/settings.html','desktop/themes.html']){
  const source=readFileSync(path,'utf8');
  for(const match of source.matchAll(/data-i18n(?:-[a-z-]+)?="([^"]+)"/g))assert.ok(catalog[match[1]],`${path}: ${match[1]}`);
  assert.match(source,/src="i18n.js" data-scripts=/);
 }
});

test('captured translators update immediately without replacing the API or user content',()=>{
 const api=create('en',catalog),{t,themeName}=api;
 api.setLocale('zh-CN');assert.equal(api.locale,'zh-CN');assert.equal(t('跟随系统'),'跟随系统');
 api.setLocale('en');assert.equal(t('跟随系统'),'System default');
 assert.equal(themeName('custom','我的主题'),'我的主题');
});
