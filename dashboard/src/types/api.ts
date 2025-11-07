// API types for siertrichain dashboard

export interface Block {
  header: BlockHeader;
  hash: string;
  transactions: Transaction[];
}

export interface BlockHeader {
  height: number;
  previous_hash: string;
  timestamp: number;
  difficulty: number;
  nonce: number;
  merkle_root: string;
}

export type Transaction = SubdivisionTx | TransferTx | CoinbaseTx;

export interface SubdivisionTx {
  Subdivision: {
    parent_hash: string;
    children: Triangle[];
    owner_address: string;
    fee: number;
    nonce: number;
    signature: number[] | null;
    public_key: number[] | null;
  };
}

export interface TransferTx {
  Transfer: {
    input_hash: string;
    new_owner: string;
    sender: string;
    fee: number;
    nonce: number;
    signature: number[] | null;
    public_key: number[] | null;
    memo: string | null;
  };
}

export interface CoinbaseTx {
  Coinbase: {
    reward_area: number;
    beneficiary_address: string;
  };
}

export interface Triangle {
  a: Point;
  b: Point;
  c: Point;
  parent_hash: string | null;
  owner: string;
}

export interface Point {
  x: number;
  y: number;
}

export interface BalanceResponse {
  triangles: string[];
  total_area: number;
}

export interface StatsResponse {
  height: number;
  difficulty: number;
  utxo_count: number;
  mempool_size: number;
  recent_blocks: RecentBlock[];
}

export interface RecentBlock {
  height: number;
  hash: string;
}

export interface TriangleInfo {
  hash: string;
  area: number;
  vertices: [number, number][];
}

export interface TransactionHistory {
  tx_hash: string;
  block_height: number;
  timestamp: number;
  tx_type: string;
}

export interface WalletResponse {
  address: string;
  public_key: string;
  private_key: string;
}

export interface MiningStatus {
  is_mining: boolean;
  blocks_mined: number;
  hashrate: number;
}

export interface PeerInfo {
  address: string;
  last_seen: number;
}

export interface NetworkInfo {
  peers_count: number;
  node_id: string;
  listening_port: number;
}
