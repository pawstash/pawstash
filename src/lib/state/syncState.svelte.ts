import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { SyncDevice, SyncStatus } from '$lib/types/sync';
import { libraryState } from './libraryState.svelte';
import { subscriptionState } from './subscriptionState.svelte';
import { accountState } from './accountState.svelte';
import {
  apiConnectSyncAccount, apiCreateSyncAccount, apiDisconnectSync, apiGetSyncStatus,
  apiChangeSyncPassword, apiCopySyncRecoveryKit, apiGetSyncRecoveryKit, apiListSyncDevices, apiLockSync,
  apiRecoverSyncAccount, apiResolveSyncConflict, apiRevokeSyncDevice, apiRunSync,
  apiSetSyncEnabled, apiUnlockSync
} from '$lib/utils/ipc';

class SyncState {
  status = $state<SyncStatus>({ configured:false, enabled:false, unlocked:false, syncing:false, revision:0, cursor:0, conflict:false });
  busy = $state(false);
  devices = $state<SyncDevice[]>([]);
  private initialized=false;
  private unlisten?:UnlistenFn;
  async init(){if(this.initialized)return;this.initialized=true;this.unlisten=await listen<SyncStatus>('sync-status-updated',({payload})=>{this.status=payload;void this.refreshLibrary()});await this.refresh()}
  async refresh(){this.status=await apiGetSyncStatus()}
  async setEnabled(enabled:boolean){return this.work(()=>apiSetSyncEnabled(enabled))}
  async create(server:string,account:string,password:string,device:string){return this.work(()=>apiCreateSyncAccount(server,account,password,device),true)}
  async connect(server:string,account:string,password:string,device:string){return this.work(()=>apiConnectSyncAccount(server,account,password,device),true)}
  async unlock(password:string){return this.work(()=>apiUnlockSync(password))}
  async lock(){return this.work(apiLockSync)}
  async disconnect(){return this.work(apiDisconnectSync,true)}
  async changePassword(currentPassword:string,newPassword:string){return this.work(()=>apiChangeSyncPassword(currentPassword,newPassword))}
  async getRecoveryKit(){this.busy=true;try{return await apiGetSyncRecoveryKit()}finally{this.busy=false}}
  async copyRecoveryKit(){this.busy=true;try{return await apiCopySyncRecoveryKit()}finally{this.busy=false}}
  async recover(recoveryKit:string,newPassword:string,device:string){return this.work(()=>apiRecoverSyncAccount(recoveryKit,newPassword,device),true)}
  async loadDevices(){this.busy=true;try{this.devices=await apiListSyncDevices();return this.devices}finally{this.busy=false}}
  async revokeDevice(deviceId:string){this.busy=true;try{this.devices=await apiRevokeSyncDevice(deviceId);return this.devices}finally{this.busy=false}}
  async sync(){return this.work(apiRunSync,true)}
  async resolve(resolution:'local'|'remote'){return this.work(()=>apiResolveSyncConflict(resolution),true)}
  private async work(call:()=>Promise<SyncStatus>,refreshLibrary=false){this.busy=true;try{this.status=await call();if(refreshLibrary)await this.refreshLibrary();return this.status}finally{this.busy=false}}
  private async refreshLibrary(){await Promise.all([libraryState.refreshCollections(),libraryState.refreshSavedKeys(),libraryState.refresh(),subscriptionState.reload(),accountState.refresh(),accountState.fetchFavorites('post', true),accountState.fetchFavorites('creator', true)])}
}
export const syncState=new SyncState();
