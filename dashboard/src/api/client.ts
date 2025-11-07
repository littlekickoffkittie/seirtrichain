// API client for siertrichain backend

import type {
  BalanceResponse,
  Block,
  MiningStatus,
  NetworkInfo,
  PeerInfo,
  RecentBlock,
  StatsResponse,
  Transaction,
  TransactionHistory,
  TriangleInfo,
  WalletResponse
} from '../types/api';

const API_BASE = '/api';

export class ApiClient {
  private async fetchJSON<T>(endpoint: string, options?: RequestInit): Promise<T> {
    const response = await fetch(`${API_BASE}${endpoint}`, {
      ...options,
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.statusText}`);
    }

    return response.json();
  }

  // Blockchain endpoints
  async getBlockchainHeight(): Promise<number> {
    return this.fetchJSON<number>('/blockchain/height');
  }

  async getBlockchainStats(): Promise<StatsResponse> {
    return this.fetchJSON<StatsResponse>('/blockchain/stats');
  }

  async getRecentBlocks(): Promise<RecentBlock[]> {
    return this.fetchJSON<RecentBlock[]>('/blockchain/blocks');
  }

  async getBlockByHash(hash: string): Promise<Block | null> {
    return this.fetchJSON<Block | null>(`/blockchain/block/${hash}`);
  }

  async getBlockByHeight(height: number): Promise<Block | null> {
    return this.fetchJSON<Block | null>(`/blockchain/block/by-height/${height}`);
  }

  // Address endpoints
  async getAddressBalance(address: string): Promise<BalanceResponse> {
    return this.fetchJSON<BalanceResponse>(`/address/${address}/balance`);
  }

  async getAddressTriangles(address: string): Promise<TriangleInfo[]> {
    return this.fetchJSON<TriangleInfo[]>(`/address/${address}/triangles`);
  }

  async getAddressHistory(address: string): Promise<TransactionHistory[]> {
    return this.fetchJSON<TransactionHistory[]>(`/address/${address}/history`);
  }

  // Transaction endpoints
  async submitTransaction(tx: Transaction): Promise<string> {
    return this.fetchJSON<string>('/transaction', {
      method: 'POST',
      body: JSON.stringify(tx),
    });
  }

  async getTransactionStatus(hash: string): Promise<Transaction | null> {
    return this.fetchJSON<Transaction | null>(`/transaction/${hash}`);
  }

  async getPendingTransactions(): Promise<Transaction[]> {
    return this.fetchJSON<Transaction[]>('/transactions/pending');
  }

  // Wallet endpoints
  async createWallet(): Promise<WalletResponse> {
    return this.fetchJSON<WalletResponse>('/wallet/create', {
      method: 'POST',
    });
  }

  async importWallet(privateKey: string): Promise<WalletResponse> {
    return this.fetchJSON<WalletResponse>('/wallet/import', {
      method: 'POST',
      body: JSON.stringify({ private_key: privateKey }),
    });
  }

  // Mining endpoints
  async getMiningStatus(): Promise<MiningStatus> {
    return this.fetchJSON<MiningStatus>('/mining/status');
  }

  async startMining(): Promise<string> {
    return this.fetchJSON<string>('/mining/start', {
      method: 'POST',
    });
  }

  async stopMining(): Promise<string> {
    return this.fetchJSON<string>('/mining/stop', {
      method: 'POST',
    });
  }

  // Network endpoints
  async getPeers(): Promise<PeerInfo[]> {
    return this.fetchJSON<PeerInfo[]>('/network/peers');
  }

  async getNetworkInfo(): Promise<NetworkInfo> {
    return this.fetchJSON<NetworkInfo>('/network/info');
  }
}

export const apiClient = new ApiClient();
