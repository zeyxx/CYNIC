import { Outlet, Link } from 'react-router-dom';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

export function Layout() {
  return (
    <>
      <nav className="topbar" aria-label="Primary">
        <Link className="brand" to="/">
          CYNIC<span>.</span>
        </Link>
        <div className="nav-links">
          <Link to="/manifest">Manifest</Link>
          <Link to="/build-log">Build Log</Link>
          <Link to="/decisions">Decisions</Link>
          <Link to="/outcomes">Outcomes</Link>
          <WalletMultiButton />
        </div>
      </nav>
      <main>
        <Outlet />
      </main>
      <footer>
        <span>TALARIA SAS</span>
        <span>CYNIC Framework</span>
        <span>One-Person Enterprise v1</span>
      </footer>
    </>
  );
}
