import { Link, useLocation } from 'react-router-dom';
import './Navigation.css';

export default function Navigation() {
  const location = useLocation();

  const isActive = (path: string) => {
    return location.pathname === path;
  };

  return (
    <nav className="navigation">
      <div className="nav-header">
        <h1 className="nav-title">🔺 SierTriChain</h1>
        <p className="nav-subtitle">Fractal Blockchain Dashboard</p>
      </div>

      <ul className="nav-links">
        <li>
          <Link to="/" className={isActive('/') ? 'active' : ''}>
            <span className="nav-icon">📊</span>
            Dashboard
          </Link>
        </li>
        <li>
          <Link to="/wallet" className={isActive('/wallet') ? 'active' : ''}>
            <span className="nav-icon">💼</span>
            Wallet
          </Link>
        </li>
        <li>
          <Link to="/blocks" className={isActive('/blocks') ? 'active' : ''}>
            <span className="nav-icon">🧱</span>
            Blocks
          </Link>
        </li>
        <li>
          <Link to="/transactions" className={isActive('/transactions') ? 'active' : ''}>
            <span className="nav-icon">📝</span>
            Transactions
          </Link>
        </li>
        <li>
          <Link to="/mining" className={isActive('/mining') ? 'active' : ''}>
            <span className="nav-icon">⛏️</span>
            Mining
          </Link>
        </li>
        <li>
          <Link to="/network" className={isActive('/network') ? 'active' : ''}>
            <span className="nav-icon">🌐</span>
            Network
          </Link>
        </li>
      </ul>

      <div className="nav-footer">
        <p>v0.1.0</p>
        <p className="text-muted">Powered by Rust 🦀</p>
      </div>
    </nav>
  );
}
