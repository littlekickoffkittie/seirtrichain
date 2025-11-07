import { useState } from 'react';
import { apiClient } from '../api/client';
import type { WalletResponse, BalanceResponse } from '../types/api';
import './PageCommon.css';

export default function Wallet() {
  const [wallet, setWallet] = useState<WalletResponse | null>(null);
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [address, setAddress] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreateWallet = async () => {
    try {
      setLoading(true);
      setError(null);
      const newWallet = await apiClient.createWallet();
      setWallet(newWallet);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create wallet');
    } finally {
      setLoading(false);
    }
  };

  const handleCheckBalance = async () => {
    if (!address.trim()) {
      setError('Please enter an address');
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const bal = await apiClient.getAddressBalance(address);
      setBalance(bal);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to check balance');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>💼 Wallet Management</h1>
        <p className="text-muted">Create wallets and check balances</p>
      </div>

      <div className="grid-2col">
        <div className="card">
          <h2>Create New Wallet</h2>
          <p className="text-muted">Generate a new wallet with keypair</p>

          <button onClick={handleCreateWallet} disabled={loading}>
            {loading ? 'Creating...' : 'Create Wallet'}
          </button>

          {wallet && (
            <div className="wallet-info">
              <div className="info-item">
                <label>Address:</label>
                <code>{wallet.address}</code>
              </div>
              <div className="info-item">
                <label>Public Key:</label>
                <code>{wallet.public_key.substring(0, 32)}...</code>
              </div>
              <div className="info-item">
                <label>Private Key:</label>
                <code className="warning">{wallet.private_key.substring(0, 32)}...</code>
                <small className="text-muted">⚠️ Keep this secret!</small>
              </div>
            </div>
          )}
        </div>

        <div className="card">
          <h2>Check Balance</h2>
          <p className="text-muted">View triangles owned by an address</p>

          <div className="form-group">
            <label>Address:</label>
            <input
              type="text"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="Enter address..."
            />
          </div>

          <button onClick={handleCheckBalance} disabled={loading}>
            {loading ? 'Checking...' : 'Check Balance'}
          </button>

          {balance && (
            <div className="balance-info">
              <div className="stat-card">
                <div className="stat-value">{balance.triangles.length}</div>
                <div className="stat-label">Triangles Owned</div>
              </div>
              <div className="stat-card">
                <div className="stat-value">{balance.total_area.toFixed(4)}</div>
                <div className="stat-label">Total Area</div>
              </div>
            </div>
          )}
        </div>
      </div>

      {error && (
        <div className="error-message">
          {error}
        </div>
      )}
    </div>
  );
}
