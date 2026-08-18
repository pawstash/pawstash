import pkg from '../../package.json';

declare const __BUILD_TIME__: string;
declare const __COMMIT_HASH__: string;

export const APP_VERSION: string = pkg.version;
export const BUILD_TIME: string = typeof __BUILD_TIME__ !== 'undefined' ? __BUILD_TIME__ : new Date().toISOString();
export const COMMIT_HASH: string = typeof __COMMIT_HASH__ !== 'undefined' ? __COMMIT_HASH__ : 'dev';
