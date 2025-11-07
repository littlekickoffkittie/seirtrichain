import { useEffect, useState } from 'react';
import { apiClient } from '../api/client';
import type { StatsResponse } from '../types/api';
import './Dashboard.css';

export default function Dashboard() {
  const [stats, setStats] = useState<StatsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadStats = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await apiClient.getBlockchainStats();
      setStats(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load stats');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 5000); // Refresh every 5 seconds
    return () => clearInterval(interval);
  }, []);

  if (loading && !stats) {
    return (
      <div className="dashboard">
        <div className="spinner-container">
          <div className="spinner"></div>
          <p>Loading dashboard...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="dashboard">
        <div className="error-message">
          <h3>Error loading dashboard</h3>
          <p>{error}</p>
          <button onClick={loadStats}>Retry</button>
        </div>
      </div>
    );
  }

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h1>Dashboard</h1>
        <p className="text-muted">Real-time blockchain statistics</p>
      </div>

      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-value">{stats?.height || 0}</div>
          <div className="stat-label">Block Height</div>
        </div>

        <div className="stat-card">
          <div className="stat-value">{stats?.difficulty || 0}</div>
          <div className="stat-label">Difficulty</div>
        </div>

        <div className="stat-card">
          <div className="stat-value">{stats?.utxo_count || 0}</div>
          <div className="stat-label">Triangle UTXOs</div>
        </div>

        <div className="stat-card">
          <div className="stat-value">{stats?.mempool_size || 0}</div>
          <div className="stat-label">Pending Transactions</div>
        </div>
      </div>

      <div className="recent-blocks-section">
        <div className="section-header">
          <h2>Recent Blocks</h2>
          <span className="badge info">Last 6 blocks</span>
        </div>

        {stats?.recent_blocks && stats.recent_blocks.length > 0 ? (
          <div className="blocks-list">
            {stats.recent_blocks.map((block) => (
              <div key={block.hash} className="block-item card">
                <div className="block-info">
                  <div className="block-height">
                    <span className="label">Height:</span>
                    <span className="value">{block.height}</span>
                  </div>
                  <div className="block-hash">
                    <span className="label">Hash:</span>
                    <code className="value">{block.hash.substring(0, 16)}...</code>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-muted">No recent blocks found</p>
        )}
      </div>

      <div className="quick-actions">
        <h2>Quick Actions</h2>
        <div className="actions-grid">
          <a href="/wallet" className="action-card card">
            <span className="action-icon">💼</span>
            <h3>Manage Wallet</h3>
            <p>Create or import wallets, check balances</p>
          </a>

          <a href="/mining" className="action-card card">
            <span className="action-icon">⛏️</span>
            <h3>Start Mining</h3>
            <p>Mine blocks and earn triangle rewards</p>
          </a>

          <a href="/blocks" className="action-card card">
            <span className="action-icon">🧱</span>
            <h3>Explore Blocks</h3>
            <p>Browse blockchain and view transactions</p>
          </a>

          <a href="/network" className="action-card card">
            <span className="action-icon">🌐</span>
            <h3>Network Status</h3>
            <p>View connected peers and network info</p>
          </a>
        </div>
      </div>
    </div>
  );
}
