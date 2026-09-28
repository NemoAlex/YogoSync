import os from 'node:os';
import path from 'node:path';
export const dataDir = process.env.YOGO_PET_HOME || path.join(os.homedir(), '.local', 'share', 'yogo-pet');
