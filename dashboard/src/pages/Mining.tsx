import { useEffect, useState } from 'react';
import { apiClient } from '../api/client';
import type { MiningStatus } from '../types/api';
import './PageCommon.css';

export default function Mining() {
  const [status, setStatus] = useState<MiningStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const loadStatus = async () => {
    try {
      const data = await apiClient.getMiningStatus();
      setStatus(data);
    } catch (err) {
      console.error('Failed to load mining status:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStatus();
    const interval = setInterval(loadStatus, 3000);
    return () => clearInterval(interval);
  }, []);

  const handleStart = async () => {
    try {
      await apiClient.startMining();
      await loadStatus();
    } catch (err) {
      console.error('Failed to start mining:', err);
    }
  };

  const handleStop = async () => {
    try {
      await apiClient.stopMining();
      await loadStatus();
    } catch (err) {
      console.error('Failed to stop mining:', err);
    }
  };

  if (loading) {
    return (
      <div className="page">
        <div className="spinner-container">
          <div className="spinner"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="page">
      <div className="page-header">
        <h1>⛏️ Mining Control</h1>
        <p className="text-muted">Manage mining operations and monitor hashrate</p>
      </div>

      <div className="card">
        <h2>Mining Status</h2>

        <div className="status-indicator">
          <span className={`status-dot ${status?.is_mining ? 'active' : 'inactive'}`}></span>
          {status?.is_mining ? 'Mining Active' : 'Mining Inactive'}
        </div>

        <div className="grid-2col" style={{ marginTop: '2rem' }}>
          <div className="stat-card">
            <div className="stat-value">{status?.blocks_mined || 0}</div>
            <div className="stat-label">Blocks Mined</div>
          </div>

          <div className="stat-card">
            <div className="stat-value">{status?.hashrate?.toFixed(2) || '0.00'} H/s</div>
            <div className="stat-label">Current Hashrate</div>
          </div>
        </div>

        <div className="mining-controls">
          <button
            onClick={handleStart}
            disabled={status?.is_mining}
          >
            Start Mining
          </button>
          <button
            className="danger"
            onClick={handleStop}
            disabled={!status?.is_mining}
          >
            Stop Mining
          </button>
        </div>

        <div className="info-item" style={{ marginTop: '2rem' }}>
          <p className="text-muted">
            ℹ️ Mining is currently not fully implemented in the API.
            This interface demonstrates the controls that will be available.
          </p>
        </div>
      </div>
    </div>
  );
}
