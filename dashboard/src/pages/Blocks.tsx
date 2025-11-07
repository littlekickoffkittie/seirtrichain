import './PageCommon.css';

export default function Blocks() {
  return (
    <div className="page">
      <div className="page-header">
        <h1>🧱 Block Explorer</h1>
        <p className="text-muted">Browse blockchain blocks and transactions</p>
      </div>

      <div className="placeholder-page card">
        <h2>Block Explorer Coming Soon</h2>
        <p className="text-muted">
          This page will display recent blocks, allow searching by height or hash,
          and show detailed transaction information.
        </p>
      </div>
    </div>
  );
}
