/// <reference types="astro/client" />

interface ImportMetaEnv {
  readonly SOROBAN_NETWORK_PASSPHRASE: string;
  readonly SOROBAN_RPC_URL: string;
  readonly SOROBAN_SOURCE_ACCOUNT: string;
}
