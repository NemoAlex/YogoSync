import vm from 'node:vm';
import {readFileSync} from 'node:fs';
export const catalog=JSON.parse(readFileSync(new URL('../desktop/locales/en.json',import.meta.url),'utf8'));
const context={window:{},document:{currentScript:null}};
vm.runInNewContext(readFileSync(new URL('../desktop/i18n.js',import.meta.url),'utf8'),context);
export const {create,resolveLocale}=context.window.YogoI18nFactory;
