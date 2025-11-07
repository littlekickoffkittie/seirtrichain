import './PageCommon.css';

export default function Transactions() {
  return (
    <div className="page">
      <div className="page-header">
        <h1>📝 Transactions</h1>
        <p className="text-muted">View pending and confirmed transactions</p>
      </div>

      <div className="placeholder-page card">
        <h2>Transaction History Coming Soon</h2>
        <p className="text-muted">
          This page will show pending transactions in the mempool and allow
          filtering confirmed transactions by address and type.
        </p>
      </div>
    </div>
  );
}
