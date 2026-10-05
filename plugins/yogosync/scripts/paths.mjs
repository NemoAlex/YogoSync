import os from 'node:os';
import path from 'node:path';
export const dataDir = process.env.YOGOSYNC_HOME || path.join(os.homedir(), '.local', 'share', 'yogosync');
